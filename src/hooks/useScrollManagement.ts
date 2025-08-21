import { useRef, useCallback, useEffect } from "react";

export interface UseScrollManagementReturn {
    messagesEndRef: React.RefObject<HTMLDivElement | null>;
    scrollContainerRef: React.RefObject<HTMLDivElement | null>;
    handleScroll: () => void;
    smartScroll: () => void;
}

export function useScrollManagement(): UseScrollManagementReturn {
    // 滚动相关状态和逻辑
    const messagesEndRef = useRef<HTMLDivElement | null>(null);
    const scrollContainerRef = useRef<HTMLDivElement | null>(null);
    const isUserScrolledUpRef = useRef(false); // 使用 Ref 来跟踪滚动状态，避免闭包问题
    const isAutoScrolling = useRef(false);
    const resizeObserverRef = useRef<ResizeObserver | null>(null);

    // 处理用户滚动事件
    const handleScroll = useCallback(() => {
        // 如果是程序触发的自动滚动，则忽略此次事件
        if (isAutoScrolling.current) {
            return;
        }

        const container = scrollContainerRef.current;
        if (container) {
            const { scrollTop, scrollHeight, clientHeight } = container;
            // 判断是否滚动到了底部，留出 10px 的容差
            const atBottom = scrollHeight - scrollTop - clientHeight < 10;

            // 直接更新 Ref 的值
            isUserScrolledUpRef.current = !atBottom;
        }
    }, []); // 依赖项为空，函数是稳定的

    // 智能滚动函数
    const smartScroll = useCallback(() => {
        // 从 Ref 读取状态，这总是最新的值
        if (isUserScrolledUpRef.current) {
            return;
        }

        const container = scrollContainerRef.current;
        if (!container) return;

        // 清理之前的观察器
        if (resizeObserverRef.current) {
            resizeObserverRef.current.disconnect();
        }

        resizeObserverRef.current = new ResizeObserver(() => {
            // 再次从 Ref 检查，确保万无一失
            if (isUserScrolledUpRef.current || !scrollContainerRef.current) {
                if (resizeObserverRef.current) {
                    resizeObserverRef.current.disconnect();
                }
                return;
            }

            isAutoScrolling.current = true;
            scrollContainerRef.current.scrollTop =
                scrollContainerRef.current.scrollHeight;

            if (resizeObserverRef.current) {
                resizeObserverRef.current.disconnect();
            }

            setTimeout(() => {
                isAutoScrolling.current = false;
            }, 100);
        });

        const lastMessageElement = container.lastElementChild;
        if (lastMessageElement) {
            resizeObserverRef.current.observe(lastMessageElement);
        }
    }, []); // 依赖项为空，函数是稳定的

    // 组件卸载时清理资源
    useEffect(() => {
        return () => {
            if (resizeObserverRef.current) {
                resizeObserverRef.current.disconnect();
                resizeObserverRef.current = null;
            }
        };
    }, []); // 只在组件卸载时清理

    return {
        messagesEndRef,
        scrollContainerRef,
        handleScroll,
        smartScroll,
    };
}