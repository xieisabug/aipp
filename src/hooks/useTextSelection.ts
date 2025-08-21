import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface UseTextSelectionReturn {
    selectedText: string;
}

export function useTextSelection(): UseTextSelectionReturn {
    // 选中文本相关状态和逻辑
    const [selectedText, setSelectedText] = useState<string>("");

    // 获取选中文本
    useEffect(() => {
        // 初始获取选中文本
        invoke<string>("get_selected_text_api").then((text) => {
            console.log("get_selected_text_api", text);
            setSelectedText(text);
        });

        // 监听选中文本变化事件
        const unsubscribe = listen<string>("get_selected_text_event", (event) => {
            console.log("get_selected_text_event", event.payload);
            setSelectedText(event.payload);
        });

        // 清理事件监听器
        return () => {
            if (unsubscribe) {
                unsubscribe.then((f) => f());
            }
        };
    }, []);

    return {
        selectedText,
    };
}