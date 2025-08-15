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
    groupRootMessageIds: Map<string, number>; // 更改：分组基准消息ID
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
    const pureGenerationGroupsAndTimestamps = useMemo(() => {
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

        // 先记录每个分组的基准消息ID（使用最顶层父分组的最小消息 ID）
        const groupRootMessageIds = new Map<string, number>();
        versionDataMap.forEach((_, versionId) => {
            const rootId = findRoot(versionId);
            
            // 如果已经计算过这个根分组，跳过
            if (groupRootMessageIds.has(rootId)) {
                return;
            }
            
            // 找到这个根分组树中所有的版本数据
            const allVersionsInGroup: Array<Pick<Version, "messages" | "parentGroupId">> = [];
            versionDataMap.forEach((versionData, vId) => {
                if (findRoot(vId) === rootId) {
                    allVersionsInGroup.push(versionData);
                }
            });
            
            // 找到最顶层的父分组（没有 parentGroupId 的那个）
            const topLevelVersions = allVersionsInGroup.filter(v => !v.parentGroupId);
            
            let groupBaseId = Infinity;
            if (topLevelVersions.length > 0) {
                // 使用顶层父分组的最小消息 ID
                for (const version of topLevelVersions) {
                    const minId = Math.min(...version.messages.map(m => m.id));
                    if (minId > 0 && minId < groupBaseId) {
                        groupBaseId = minId;
                    }
                }
            } else {
                // 如果没有顶层版本（理论上不应该发生），使用所有版本的最小 ID
                for (const version of allVersionsInGroup) {
                    const minId = Math.min(...version.messages.map(m => m.id));
                    if (minId > 0 && minId < groupBaseId) {
                        groupBaseId = minId;
                    }
                }
            }
            
            const finalBaseId = groupBaseId === Infinity ? 0 : groupBaseId;
            groupRootMessageIds.set(rootId, finalBaseId);
        });

        const groups = new Map<string, GenerationGroup>();
        versionDataMap.forEach((data, versionId) => {
            const rootId = findRoot(versionId);
            const group = groups.get(rootId) ?? { versions: [] };
            
            // 使用分组的根消息ID作为所有版本的基准，确保分组排序稳定
            const groupBaseMessageId = groupRootMessageIds.get(rootId) || 0;
            
            group.versions.push({
                ...data,
                versionId,
                timestamp: new Date(groupBaseMessageId),
            });
            groups.set(rootId, group);
        });

        // 步骤 3: 对每个组内的版本进行排序
        groups.forEach((group) => {            
            group.versions.sort((a, b) => {
                // 根版本（没有 parentGroupId）优先
                if (!a.parentGroupId && b.parentGroupId) return -1;
                if (a.parentGroupId && !b.parentGroupId) return 1;
                
                // 都是根版本或都是子版本时，按消息的最小 ID 排序（ID 是自增的，更可靠）
                const aMinId = Math.min(...a.messages.map((m) => m.id));
                const bMinId = Math.min(...b.messages.map((m) => m.id));
                
                return aMinId - bMinId;
            });
            
            group.versions.forEach(version => {
                version.messages.sort((a, b) => a.id - b.id);
            });
        });

        return { groups, groupRootMessageIds };
    }, [allDisplayMessages, groupMergeMap]);

    const pureGenerationGroups = pureGenerationGroupsAndTimestamps.groups;
    const groupRootMessageIds = pureGenerationGroupsAndTimestamps.groupRootMessageIds;

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
        groupRootMessageIds: groupRootMessageIds,
        handleGenerationVersionChange,
        getMessageVersionInfo,
        getGenerationGroupControl,
    };
}
