import { useState, useCallback } from "react";

export interface UseReasoningExpandReturn {
    reasoningExpandStates: Map<number, boolean>;
    toggleReasoningExpand: (messageId: number) => void;
}

export function useReasoningExpand(): UseReasoningExpandReturn {
    // 管理每个 reasoning 消息的展开状态
    const [reasoningExpandStates, setReasoningExpandStates] = useState<
        Map<number, boolean>
    >(new Map());

    // 切换 reasoning 消息的展开状态
    const toggleReasoningExpand = useCallback((messageId: number) => {
        setReasoningExpandStates((prev) => {
            const newMap = new Map(prev);
            newMap.set(messageId, !newMap.get(messageId));
            return newMap;
        });
    }, []);

    return {
        reasoningExpandStates,
        toggleReasoningExpand,
    };
}