// src/hooks/useMessageGroups.ts

import { useState, useMemo, useCallback, useEffect, useRef } from "react";
import { Message } from "../data/Conversation";

// 钩子的输入参数
interface UseMessageGroupsProps {
    allDisplayMessages: Message[];
    groupMergeMap: Map<string, string>;
}

// 版本定义
interface Version {
    messages: Message[];
    timestamp: Date;
    versionId: string;
    parentGroupId?: string;
    isPlaceholder?: boolean;
}

// 组的定义
interface GenerationGroup {
    versions: Version[];
}

// 钩子返回的类型
interface UseMessageGroupsReturn {
    generationGroups: Map<string, GenerationGroup>;
    selectedVersions: Map<string, number>;
    handleGenerationVersionChange: (
        groupId: string,
        versionIndex: number,
    ) => void;
    getMessageVersionInfo: (message: Message) => { shouldShow: boolean } | null;
    getGenerationGroupControl: (message: Message) => {
        currentVersion: number;
        totalVersions: number;
        groupId: string;
    } | null;
}

export function useMessageGroups({
    allDisplayMessages,
    groupMergeMap,
}: UseMessageGroupsProps): UseMessageGroupsReturn {
    const [selectedVersions, setSelectedVersions] = useState<
        Map<string, number>
    >(new Map());
    const userSwitchRef = useRef(false);

    // =================================================================================
    // 优化点 1: 核心计算逻辑重构
    // - 将多个循环合并为一次遍历，提升效率
    // - 使用更清晰的步骤: 1.收集版本 -> 2.分组 -> 3.排序
    // - 不再有副作用，成为一个纯粹的计算过程
    // =================================================================================
    const pureGenerationGroups = useMemo(() => {
        // 步骤 1: 一次遍历，收集所有版本数据和父子关系
        const versionDataMap = new Map<
            string,
            Pick<Version, "messages" | "parentGroupId">
        >();
        const groupToParentMap = new Map<string, string>();

        for (const msg of allDisplayMessages) {
            if (!msg.generation_group_id) {
                continue;
            }
            const versionKey = msg.generation_group_id;
            if (msg.parent_group_id) {
                groupToParentMap.set(versionKey, msg.parent_group_id);
            }
            const versionData = versionDataMap.get(versionKey) ?? {
                messages: [],
            };
            versionData.messages.push(msg);
            if (msg.parent_group_id)
                versionData.parentGroupId = msg.parent_group_id;
            versionDataMap.set(versionKey, versionData);
        }

        // 步骤 2: 高效查找根节点并分组
        const groupToRootCache = new Map<string, string>();
        const findRoot = (groupId: string): string => {
            if (groupToRootCache.has(groupId))
                return groupToRootCache.get(groupId)!;

            const mergedId = groupMergeMap.get(groupId) || groupId;
            const parentId = groupToParentMap.get(mergedId);

            if (!parentId) {
                groupToRootCache.set(groupId, mergedId);
                return mergedId;
            }
            const root = findRoot(parentId);
            groupToRootCache.set(groupId, root); // 路径压缩
            return root;
        };

        const groups = new Map<string, GenerationGroup>();
        versionDataMap.forEach((data, versionId) => {
            const rootId = findRoot(versionId);
            const group = groups.get(rootId) ?? { versions: [] };
            group.versions.push({
                ...data,
                versionId,
                timestamp: new Date(
                    Math.min(
                        ...data.messages
                            .map((m) => new Date(m.created_time || 0).getTime())
                            .filter((t) => t > 0),
                    ) || 0,
                ),
            });
            groups.set(rootId, group);
        });

        // 步骤 3: 对每个组内的版本进行排序
        groups.forEach((group) => {
            group.versions.sort((a, b) => {
                if (!a.parentGroupId && b.parentGroupId) return -1;
                if (a.parentGroupId && !b.parentGroupId) return 1;
                return a.timestamp.getTime() - b.timestamp.getTime();
            });
        });

        return groups;
    }, [allDisplayMessages, groupMergeMap]);

    // =================================================================================
    // 优化点 2: 逻辑分离
    // - 将视图相关的占位符逻辑从核心计算中分离
    // - 这个 memo 依赖于纯数据和用户选择，职责更清晰
    // =================================================================================
    const generationGroups = useMemo(() => {
        const newGroups = new Map<string, GenerationGroup>();
        pureGenerationGroups.forEach((group, groupId) => {
            const versions = [...group.versions];
            const selectedVersionIndex = selectedVersions.get(groupId);

            if (
                selectedVersionIndex !== undefined &&
                selectedVersionIndex >= versions.length
            ) {
                const placeholderCount =
                    selectedVersionIndex - versions.length + 1;
                for (let i = 0; i < placeholderCount; i++) {
                    versions.push({
                        messages: [],
                        versionId: `placeholder_${groupId}_${versions.length + i}`,
                        timestamp: new Date(),
                        parentGroupId: groupId,
                        isPlaceholder: true,
                    });
                }
            }
            newGroups.set(groupId, { ...group, versions });
        });
        return newGroups;
    }, [pureGenerationGroups, selectedVersions]);

    // =================================================================================
    // 优化点 3: 遵循 React 实践，将副作用移入 useEffect
    // - 自动设置新分组的默认选中版本
    // =================================================================================
    useEffect(() => {
        const newSelections = new Map<string, number>();
        pureGenerationGroups.forEach((group, groupId) => {
            if (!selectedVersions.has(groupId) && group.versions.length > 0) {
                newSelections.set(groupId, group.versions.length - 1);
            }
        });
        if (newSelections.size > 0) {
            setSelectedVersions((prev) => new Map([...prev, ...newSelections]));
        }
    }, [pureGenerationGroups, selectedVersions]);

    // 自动切换到最新版本的逻辑 (保持不变，但现在依赖于更新后的 generationGroups)
    useEffect(() => {
        if (userSwitchRef.current) {
            userSwitchRef.current = false;
            return;
        }
        generationGroups.forEach((group, groupId) => {
            const currentVersionIndex = selectedVersions.get(groupId);
            const maxVersionIndex = group.versions.length - 1;
            if (
                currentVersionIndex !== undefined &&
                currentVersionIndex < maxVersionIndex &&
                !group.versions[maxVersionIndex].isPlaceholder
            ) {
                setSelectedVersions((prev) =>
                    new Map(prev).set(groupId, maxVersionIndex),
                );
            }
        });
    }, [generationGroups, selectedVersions]);

    // =================================================================================
    // 优化点 4: 性能优化
    // - 创建查找表，让消息找其根组的耗时从 O(N) 降到 O(1)
    // =================================================================================
    const messageIdToRootGroupIdMap = useMemo(() => {
        const map = new Map<string, string>();
        pureGenerationGroups.forEach((group, rootId) => {
            group.versions.forEach((version) => {
                version.messages.forEach((message) => {
                    map.set(message.id.toString(), rootId);
                });
            });
        });
        return map;
    }, [pureGenerationGroups]);

    const handleGenerationVersionChange = useCallback(
        (groupId: string, versionIndex: number) => {
            userSwitchRef.current = true;
            setSelectedVersions((prev) =>
                new Map(prev).set(groupId, versionIndex),
            );
        },
        [],
    );

    const isLastInGenerationGroup = useCallback(
        (message: Message): boolean => {
            const rootGroupId = messageIdToRootGroupIdMap.get(
                message.id.toString(),
            );
            if (!rootGroupId) return false;

            const group = generationGroups.get(rootGroupId);
            if (!group || group.versions.length === 0) return false;

            const selectedVersionIndex =
                selectedVersions.get(rootGroupId) ?? group.versions.length - 1;
            const currentVersionData = group.versions[selectedVersionIndex];
            const lastMessageInGroup =
                currentVersionData?.messages[
                    currentVersionData.messages.length - 1
                ];

            return lastMessageInGroup?.id === message.id;
        },
        [generationGroups, selectedVersions, messageIdToRootGroupIdMap],
    );

    const getGenerationGroupControl = useCallback(
        (message: Message) => {
            if (!isLastInGenerationGroup(message)) return null;

            const rootGroupId = messageIdToRootGroupIdMap.get(
                message.id.toString(),
            );
            if (!rootGroupId) return null;

            const group = generationGroups.get(rootGroupId);
            if (!group || group.versions.length <= 1) return null;

            const selectedVersionIndex =
                selectedVersions.get(rootGroupId) ?? group.versions.length - 1;
            return {
                currentVersion: selectedVersionIndex + 1,
                totalVersions: group.versions.length,
                groupId: rootGroupId,
            };
        },
        [
            generationGroups,
            selectedVersions,
            isLastInGenerationGroup,
            messageIdToRootGroupIdMap,
        ],
    );

    const getMessageVersionInfo = useCallback(
        (message: Message) => {
            const rootGroupId = messageIdToRootGroupIdMap.get(
                message.id.toString(),
            );
            if (!rootGroupId) return null;

            const group = generationGroups.get(rootGroupId);
            if (!group) return null;

            const selectedVersionIndex =
                selectedVersions.get(rootGroupId) ?? group.versions.length - 1;
            const selectedVersionData = group.versions[selectedVersionIndex];
            if (!selectedVersionData) return null;

            if (selectedVersionData.isPlaceholder) {
                return { shouldShow: false };
            }

            const isMessageInSelectedVersion =
                selectedVersionData.messages.some(
                    (msg) => msg.id === message.id,
                );
            return { shouldShow: isMessageInSelectedVersion };
        },
        [generationGroups, selectedVersions, messageIdToRootGroupIdMap],
    );

    return {
        generationGroups,
        selectedVersions,
        handleGenerationVersionChange,
        getMessageVersionInfo,
        getGenerationGroupControl,
    };
}
