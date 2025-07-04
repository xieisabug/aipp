import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import React, { useState, useCallback, useEffect, useRef } from "react";
import SyntaxHighlighter from "react-syntax-highlighter";
// srcery   railscasts   nnfx-dark    atelier-estuary-dark
import { srcery } from "react-syntax-highlighter/dist/esm/styles/hljs";
import IconButton from "./IconButton";
import Ok from "../assets/ok.svg?react";
import Copy from "../assets/copy.svg?react";
import Run from "../assets/run.svg?react";

const CodeBlock = React.memo(({ language, children, onCodeRun }: { language: string, children: string, onCodeRun: (lang: string, code: string) => void }) => {
    const [copyIconState, setCopyIconState] = useState<'copy' | 'ok'>('copy');

    const handleCopy = useCallback(() => {
        const code = String(children).replace(/\n$/, '');
        writeText(code);
        setCopyIconState('ok');
    }, [children]);

    useEffect(() => {
        if (copyIconState === 'ok') {
            const timer = setTimeout(() => {
                setCopyIconState('copy');
            }, 1500);

            return () => clearTimeout(timer);
        }
    }, [copyIconState]);

    //================ 300ms 节流 + 增量高亮 ==================
    const [highlightedCode, setHighlightedCode] = useState<string>(String(children));
    const lastUpdateRef = useRef<number>(Date.now());
    const throttleTimer = useRef<NodeJS.Timeout | null>(null);

    useEffect(() => {
        const now = Date.now();
        const newCode = String(children);

        // 如果距离上次高亮超过 300ms 或代码增量超过 10 行，则立即更新
        const lineDiff = newCode.split("\n").length - highlightedCode.split("\n").length;
        if (now - lastUpdateRef.current > 300 || lineDiff >= 10) {
            lastUpdateRef.current = now;
            setHighlightedCode(newCode);
        } else {
            if (throttleTimer.current) clearTimeout(throttleTimer.current);
            throttleTimer.current = setTimeout(() => {
                lastUpdateRef.current = Date.now();
                setHighlightedCode(String(children));
            }, 300);
        }

        return () => {
            if (throttleTimer.current) clearTimeout(throttleTimer.current);
        };
    }, [children, language]);

    return (
        <div className="relative rounded-lg overflow-hidden group/codeblock">
            <div className="absolute right-2 top-2 flex bg-white/90 opacity-0 group-hover/codeblock:opacity-100 hover:opacity-100 transition-opacity duration-200 rounded-md p-1 backdrop-blur-sm">
                <IconButton
                    icon={copyIconState === 'copy' ? <Copy fill="black"/> : <Ok fill="black" />}
                    onClick={handleCopy}
                />
                <IconButton icon={<Run fill="black" />} onClick={() => onCodeRun(language, String(children).replace(/\n$/, ''))} />
            </div>
            {/* 渲染最近一次节流后的代码 */}
            <SyntaxHighlighter
                PreTag="div"
                children={highlightedCode.replace(/\n$/, '')}
                language={language}
                style={srcery}
            />
            {/* 如果仍有未高亮的追加内容（用户正在流式输出），附加普通文本 */}
            {highlightedCode !== String(children) && (
                <pre className="overflow-auto text-sm bg-transparent">
                    <code>{String(children).slice(highlightedCode.length)}</code>
                </pre>
            )}
        </div>
    );
});

export default CodeBlock;