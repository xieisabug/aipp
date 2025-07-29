// src/hooks/useMessageGroups.ts

import { useState, useMemo, useCallback, useEffect, useRef } from 'react';
import { Message } from '../data/Conversation';

// 钩子的输入参数
interface UseMessageGroupsProps {
    allDisplayMessages: Message[];
    groupMergeMap: Map<string, string>;
}

// 钩子返回的类型
interface UseMessageGroupsReturn {
    generationGroups: Map<string, {
        messages: Message[];
        versions: Array<{
            reasoning?: Message;
            response?: Message;
            timestamp: Date;
            versionId: string;
            parentGroupId?: string;
            isPlaceholder?: boolean;
        }>;
    }>;
    selectedVersions: Map<string, number>;
    handleGenerationVersionChange: (groupId: string, versionIndex: number) => void;
    getMessageVersionInfo: (message: Message) => { shouldShow: boolean } | null;
    getGenerationGroupControl: (message: Message) => {
        currentVersion: number;
        totalVersions: number;
        groupId: string;
    } | null;
}

export function useMessageGroups({ allDisplayMessages, groupMergeMap }: UseMessageGroupsProps): UseMessageGroupsReturn {
    // 1. 将版本状态管理移入钩子
    const [selectedVersions, setSelectedVersions] = useState<Map<string, number>>(new Map());
    const userSwitchRef = useRef(false);

    // 2. 优化后的 generationGroups 计算逻辑
    const generationGroups = useMemo(() => {
        const startTime = performance.now();
        console.log(`[useMessageGroups] 开始计算 generationGroups，消息数量: ${allDisplayMessages.length}`);
        
        const groups = new Map<string, {
            messages: Message[],
            versions: Array<{
                reasoning?: Message,
                response?: Message,
                timestamp: Date,
                versionId: string,
                parentGroupId?: string,
                isPlaceholder?: boolean
            }>
        }>();
        if (allDisplayMessages.length === 0) {
            const duration = performance.now() - startTime;
            console.log(`[useMessageGroups] 计算完成，耗时: ${duration.toFixed(2)}ms（空消息列表）`);
            return groups;
        }

        // --- 算法优化部分 ---
        // 步骤 A: 预处理，构建父子关系图和查找根节点
        const groupToParentMap = new Map<string, string>();
        allDisplayMessages.forEach(msg => {
            if (msg.generation_group_id && msg.parent_group_id) {
                groupToParentMap.set(msg.generation_group_id, msg.parent_group_id);
            }
        });

        // 步骤 B: 高效查找每个组的根节点，并缓存结果
        const groupToRootCache = new Map<string, string>();
        const findRoot = (groupId: string): string => {
            if (groupToRootCache.has(groupId)) {
                return groupToRootCache.get(groupId)!;
            }
            // 优先处理合并关系
            let current = groupMergeMap.get(groupId) || groupId;
            let parent = groupToParentMap.get(current);
            
            // 向上追溯
            while (parent) {
                current = parent;
                parent = groupToParentMap.get(current);
            }
            groupToRootCache.set(groupId, current);
            return current;
        };

        // 步骤 C: 将消息分配到其根组
        allDisplayMessages.forEach(msg => {
            if (msg.generation_group_id && (msg.message_type === 'reasoning' || msg.message_type === 'response')) {
                const rootGroupId = findRoot(msg.generation_group_id);
                if (!groups.has(rootGroupId)) {
                    groups.set(rootGroupId, { messages: [], versions: [] });
                }
                groups.get(rootGroupId)!.messages.push(msg);
            }
        });
        // --- 算法优化结束 ---

        // 步骤 D: 构建版本信息 (这部分逻辑与原来保持一致)
        groups.forEach((group, groupId) => {
            const versionMap = new Map<string, {reasoning?: Message, response?: Message, parentGroupId?: string}>();
            
            group.messages.forEach(msg => {
                const versionKey = msg.generation_group_id!;
                if (!versionMap.has(versionKey)) {
                    versionMap.set(versionKey, { parentGroupId: msg.parent_group_id || undefined });
                }
                const version = versionMap.get(versionKey)!;
                if (msg.message_type === 'reasoning') {
                    version.reasoning = msg;
                } else if (msg.message_type === 'response') {
                    version.response = msg;
                }
            });
            
            const versions = Array.from(versionMap.entries())
                .map(([versionId, versionData]) => ({
                    ...versionData,
                    versionId,
                    timestamp: new Date(versionData.reasoning?.created_time || versionData.response?.created_time || new Date())
                }))
                .sort((a, b) => {
                    if (!a.parentGroupId && b.parentGroupId) return -1;
                    if (a.parentGroupId && !b.parentGroupId) return 1;
                    return a.timestamp.getTime() - b.timestamp.getTime();
                });
            
            group.versions = versions;
            
            // 检查是否需要添加占位符版本
            const selectedVersionIndex = selectedVersions.get(groupId);
            if (selectedVersionIndex !== undefined && selectedVersionIndex >= versions.length) {
                const placeholderCount = selectedVersionIndex - versions.length + 1;
                for (let i = 0; i < placeholderCount; i++) {
                    const placeholderVersion = {
                        versionId: `placeholder_${groupId}_${versions.length + i}`,
                        timestamp: new Date(),
                        parentGroupId: groupId,
                        isPlaceholder: true
                    };
                    versions.push(placeholderVersion);
                }
                group.versions = versions;
            }
            
            // 设置默认选中最新版本
            if (!selectedVersions.has(groupId) && versions.length > 0) {
                const defaultVersionIndex = versions.length - 1;
                setSelectedVersions(prev => new Map(prev).set(groupId, defaultVersionIndex));
            }
        });
        
        const duration = performance.now() - startTime;
        console.log(`[useMessageGroups] 计算完成，耗时: ${duration.toFixed(2)}ms，生成组数: ${groups.size}`);
        
        return groups;
    }, [allDisplayMessages, groupMergeMap, selectedVersions]);

    // 3. 将相关的回调和辅助函数移入钩子
    const handleGenerationVersionChange = useCallback((groupId: string, versionIndex: number) => {
        userSwitchRef.current = true;
        setSelectedVersions(prev => new Map(prev).set(groupId, versionIndex));
    }, []);

    // 自动切换到最新版本的逻辑
    useEffect(() => {
        if (userSwitchRef.current) {
            userSwitchRef.current = false;
            return;
        }
        generationGroups.forEach((group, groupId) => {
            const currentVersionIndex = selectedVersions.get(groupId);
            const maxVersionIndex = group.versions.length - 1;
            if (currentVersionIndex !== undefined && currentVersionIndex < maxVersionIndex) {
                setSelectedVersions(prev => new Map(prev).set(groupId, maxVersionIndex));
            }
        });
    }, [generationGroups, selectedVersions]);

    const isLastInGenerationGroup = useCallback((message: Message): boolean => {
        if (!message.generation_group_id || (message.message_type !== 'reasoning' && message.message_type !== 'response')) {
            return false;
        }
        
        let rootGroupId: string | null = null;
        for (const [groupId, group] of generationGroups.entries()) {
            if (group.messages.some(msg => msg.id === message.id)) {
                rootGroupId = groupId;
                break;
            }
        }
        
        if (!rootGroupId) return false;
        
        const group = generationGroups.get(rootGroupId);
        if (!group || group.versions.length === 0) return false;
        
        const selectedVersionIndex = selectedVersions.get(rootGroupId) ?? group.versions.length - 1;
        const currentVersionData = group.versions[selectedVersionIndex];
        
        const lastMessageInGroup = currentVersionData?.response || currentVersionData?.reasoning;
        
        return lastMessageInGroup?.id === message.id;
    }, [generationGroups, selectedVersions]);

    const getGenerationGroupControl = useCallback((message: Message) => {
        if (!message.generation_group_id || !isLastInGenerationGroup(message)) {
            return null;
        }
        
        let rootGroupId: string | null = null;
        for (const [groupId, group] of generationGroups.entries()) {
            if (group.messages.some(msg => msg.id === message.id)) {
                rootGroupId = groupId;
                break;
            }
        }
        
        if (!rootGroupId) return null;
        
        const group = generationGroups.get(rootGroupId);
        if (!group || group.versions.length <= 1) return null;
        
        const selectedVersionIndex = selectedVersions.get(rootGroupId) ?? group.versions.length - 1;
        
        return {
            currentVersion: selectedVersionIndex + 1,
            totalVersions: group.versions.length,
            groupId: rootGroupId
        };
    }, [generationGroups, selectedVersions, isLastInGenerationGroup]);

    const getMessageVersionInfo = useCallback((message: Message) => {
        if (!message.generation_group_id || (message.message_type !== 'reasoning' && message.message_type !== 'response')) {
            return null;
        }
        
        let rootGroupId: string | null = null;
        for (const [groupId, group] of generationGroups.entries()) {
            if (group.messages.some(msg => msg.id === message.id)) {
                rootGroupId = groupId;
                break;
            }
        }
        
        if (!rootGroupId) return null;
        
        const group = generationGroups.get(rootGroupId);
        if (!group || group.versions.length === 0) return null;
        
        const selectedVersionIndex = selectedVersions.get(rootGroupId) ?? group.versions.length - 1;
        const selectedVersionData = group.versions[selectedVersionIndex];
        
        if (!selectedVersionData) return null;
        
        if (selectedVersionData.isPlaceholder) {
            return {
                shouldShow: false
            };
        }
        
        const isMessageInSelectedVersion = selectedVersionData.reasoning?.id === message.id || 
                                         selectedVersionData.response?.id === message.id;
        
        return {
            shouldShow: isMessageInSelectedVersion
        };
    }, [generationGroups, selectedVersions]);

    // 4. 返回钩子需要暴露的所有状态和函数
    return {
        generationGroups,
        selectedVersions,
        handleGenerationVersionChange,
        getMessageVersionInfo,
        getGenerationGroupControl,
    };
}
