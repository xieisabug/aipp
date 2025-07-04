import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import React, { useState, useCallback, useEffect, useRef } from "react";
import 'highlight.js/styles/github-dark.css';
import IconButton from "./IconButton";
import Ok from "../assets/ok.svg?react";
import Copy from "../assets/copy.svg?react";
import Run from "../assets/run.svg?react";

const CodeBlock = React.memo(({ language, children, onCodeRun }: { language: string, children: React.ReactNode, onCodeRun: (lang: string, code: string) => void }) => {
    const [copyIconState, setCopyIconState] = useState<'copy' | 'ok'>('copy');
    const codeRef = useRef<HTMLElement>(null);

    const getCodeString = useCallback(() => {
        return codeRef.current?.innerText ?? '';
    }, []);

    const handleCopy = useCallback(() => {
        writeText(getCodeString());
        setCopyIconState('ok');
    }, [getCodeString]);

    useEffect(() => {
        if (copyIconState === 'ok') {
            const timer = setTimeout(() => {
                setCopyIconState('copy');
            }, 1500);

            return () => clearTimeout(timer);
        }
    }, [copyIconState]);

    // 不再在客户端动态高亮，直接渲染 rehype-highlight 生成的元素

    return (
        <div className="relative rounded-lg overflow-hidden group/codeblock">
            <div className="absolute right-2 top-2 flex bg-white/90 opacity-0 group-hover/codeblock:opacity-100 hover:opacity-100 transition-opacity duration-200 rounded-md p-1 backdrop-blur-sm">
                <IconButton
                    icon={copyIconState === 'copy' ? <Copy fill="black" /> : <Ok fill="black" />}
                    onClick={handleCopy}
                />
                <IconButton icon={<Run fill="black" />} onClick={() => onCodeRun(language, getCodeString())} />
            </div>

            <code ref={codeRef} className={`hljs language-${language}`}>
                {children}
            </code>
        </div>
    );
});

export default CodeBlock;