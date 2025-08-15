import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { AssistantListItem } from '../data/Assistant';

interface UseAssistantListListenerOptions {
    onAssistantListChanged: (assistantList: AssistantListItem[]) => void;
    enabled?: boolean;
}

/**
 * 监听助手列表变化的自定义Hook
 * 当后端发送 assistant_list_changed 事件时，自动重新获取助手列表并调用回调
 */
export function useAssistantListListener({ onAssistantListChanged, enabled = true }: UseAssistantListListenerOptions) {
    useEffect(() => {
        if (!enabled) return;

        const unsubscribe = listen("assistant_list_changed", async () => {
            try {
                const assistantList = await invoke<AssistantListItem[]>("get_assistants");
                onAssistantListChanged(assistantList);
            } catch (error) {
                console.error("Failed to fetch assistant list:", error);
            }
        });

        return () => {
            if (unsubscribe) {
                unsubscribe.then((f) => f());
            }
        };
    }, [onAssistantListChanged, enabled]);
}