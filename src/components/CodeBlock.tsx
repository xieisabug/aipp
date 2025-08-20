import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import React, { useState, useCallback, useEffect, useRef } from "react";
import IconButton from "./IconButton";
import Ok from "../assets/ok.svg?react";
import Copy from "../assets/copy.svg?react";
import Run from "../assets/run.svg?react";
import CodeBlockEventManager from "../utils/codeBlockEventManager";
import { useCodeTheme } from "../hooks/useCodeTheme";
import { listen } from "@tauri-apps/api/event";

const BUTTON_HEIGHT = 40;
const TOP_OFFSET = 8;
const RIGHT_OFFSET = 8;
const FIXED_BUTTON_TOP_OFFSET = 90;

const CodeBlock = React.memo(
    ({
        language,
        children,
        onCodeRun,
    }: {
        language: string;
        children: React.ReactNode;
        onCodeRun: (lang: string, code: string) => void;
    }) => {
        const [copyIconState, setCopyIconState] = useState<"copy" | "ok">("copy");
        const [shouldShowFixed, setShouldShowFixed] = useState(false);
        const [fixedButtonPosition, setFixedButtonPosition] = useState({ top: 0, right: 0 });
        const codeRef = useRef<HTMLElement>(null);
        const containerRef = useRef<HTMLDivElement>(null);
        const eventManager = CodeBlockEventManager.getInstance();

        // 获取当前主题信息
        const { currentTheme } = useCodeTheme();
        const [forceUpdate, setForceUpdate] = useState(0);

        const getCodeString = useCallback(() => {
            return codeRef.current?.innerText ?? "";
        }, []);

        const handleCopy = useCallback(() => {
            writeText(getCodeString());
            setCopyIconState("ok");
        }, [getCodeString]);

        useEffect(() => {
            if (copyIconState === "ok") {
                const timer = setTimeout(() => {
                    setCopyIconState("copy");
                }, 1500);

                return () => clearTimeout(timer);
            }
        }, [copyIconState]);

        // 处理滚动和鼠标移动的回调函数
        const handleScroll = useCallback(() => {
            if (!containerRef.current) return;

            const rect = containerRef.current.getBoundingClientRect();
            const viewportHeight = window.innerHeight;
            const viewportWidth = window.innerWidth;
            const mousePosition = eventManager.getMousePosition();

            // 代码块在视窗中可见
            const isCodeBlockVisible = rect.top < viewportHeight && rect.bottom > 0;

            // 原始按钮区域（代码块顶部）是否可见
            const originalButtonTop = rect.top;
            const originalButtonBottom = rect.top + BUTTON_HEIGHT;
            const isOriginalButtonVisible = originalButtonTop >= 0 && originalButtonBottom <= viewportHeight;

            // 鼠标是否在 CodeBlock 内
            const mouseInCodeBlock =
                mousePosition.x >= rect.left &&
                mousePosition.x <= rect.right &&
                mousePosition.y >= rect.top &&
                mousePosition.y <= rect.bottom;

            // 只有在以下条件都满足时才显示固定按钮：
            // 1. 鼠标在代码块内
            // 2. 代码块部分可见
            // 3. 原始按钮不可见（被滚动出视窗）
            const shouldShow = mouseInCodeBlock && isCodeBlockVisible && !isOriginalButtonVisible;
            setShouldShowFixed(shouldShow);

            if (shouldShow) {
                // 计算固定按钮的位置：在代码块可视区域的右上角
                const visibleTop = Math.max(rect.top, 0);
                const visibleRight = Math.min(rect.right, viewportWidth);

                setFixedButtonPosition({
                    top: visibleTop + TOP_OFFSET,
                    right: viewportWidth - visibleRight + RIGHT_OFFSET,
                });
            }
        }, [eventManager]);

        const handleMouseMove = useCallback(
            (_e: MouseEvent) => {
                // 鼠标移动时重新计算是否需要显示固定按钮
                handleScroll();
            },
            [handleScroll]
        );

        // 使用全局事件管理器注册事件监听
        useEffect(() => {
            if (!containerRef.current) return;

            eventManager.register(containerRef.current, {
                onScroll: handleScroll,
                onMouseMove: handleMouseMove,
            });

            return () => {
                if (containerRef.current) {
                    eventManager.unregister(containerRef.current);
                }
            };
        }, [eventManager, handleScroll, handleMouseMove]);

        // 监听主题变化事件
        useEffect(() => {
            const unlistenThemeChange = listen("theme-changed", async (event) => {
                console.log("CodeBlock: Theme change event received:", event.payload);
                // 强制重新渲染以应用新主题
                setForceUpdate((prev) => prev + 1);
            });

            return () => {
                unlistenThemeChange.then((f) => f());
            };
        }, []);

        // 不再在客户端动态高亮，直接渲染 rehype-highlight 生成的元素

        const ButtonGroup = () => (
            <div className="flex bg-white/90 opacity-0 group-hover/codeblock:opacity-100 hover:opacity-100 transition-opacity duration-200 rounded-md p-1 backdrop-blur-sm">
                <IconButton
                    icon={copyIconState === "copy" ? <Copy fill="black" /> : <Ok fill="black" />}
                    onClick={handleCopy}
                />
                <IconButton icon={<Run fill="black" />} onClick={() => onCodeRun(language, getCodeString())} />
            </div>
        );

        return (
            <div
                ref={containerRef}
                className="relative rounded-lg overflow-hidden group/codeblock prose-code:text-sm"
                data-theme={currentTheme}
                data-force-update={forceUpdate}
            >
                {/* 普通状态下的按钮 */}
                <div className="absolute right-2 top-2 z-10">
                    <ButtonGroup />
                </div>

                {/* 滚动时的固定按钮 */}
                {shouldShowFixed && (
                    <div
                        className="fixed z-50"
                        style={{
                            top: `${fixedButtonPosition.top + FIXED_BUTTON_TOP_OFFSET}px`,
                            right: `${fixedButtonPosition.right}px`,
                        }}
                    >
                        <ButtonGroup />
                    </div>
                )}

                <code ref={codeRef} className={`hljs language-${language}`}>
                    {children}
                </code>
            </div>
        );
    }
);

export default CodeBlock;
