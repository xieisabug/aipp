import React, { useCallback, useEffect, useMemo, useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import ReactMarkdown, { Components } from "react-markdown";
import remarkMath from "remark-math";
import remarkBreaks from "remark-breaks";
import rehypeRaw from "rehype-raw";
import rehypeKatex from "rehype-katex";
import rehypeHighlight from "rehype-highlight";
import rehypeSanitize, { defaultSchema } from "rehype-sanitize";
import remarkCustomCompenent from "@/react-markdown/remarkCustomComponent";
import TipsComponent from "@/react-markdown/components/TipsComponent";
import IconButton from "./IconButton";
import Copy from "../assets/copy.svg?react";
import Ok from "../assets/ok.svg?react";
import Refresh from "../assets/refresh.svg?react";
import CodeBlock from "./CodeBlock";
import MessageFileAttachment from "./MessageFileAttachment";
import MessageWebContent from "./conversation/MessageWebContent";

interface CustomComponents extends Components {
    fileattachment: React.ElementType;
    bangwebtomarkdown: React.ElementType;
    bangweb: React.ElementType;
}

const MessageItem = React.memo(
    ({ message, onCodeRun, onMessageRegenerate }: any) => {
        const [copyIconState, setCopyIconState] = useState<"copy" | "ok">(
            "copy",
        );
        const [currentMessageIndex, setCurrentMessageIndex] = useState<number>(
            message.regenerate?.length > 0 ? message.regenerate.length + 1 : -1,
        );
        // 仅记录当前查看的版本索引，真实内容按需计算，避免在流式更新时重复 setState
        const displayedContent = useMemo(() => {
            if (message.regenerate?.length > 0) {
                // currentMessageIndex 为 1 表示原始回复
                if (currentMessageIndex === 1) return message.content;
                // 其它值指向 regenerate 数组（index 从 0 开始）
                return message.regenerate[currentMessageIndex - 2]?.content ?? message.content;
            }
            return message.content;
        }, [message.content, message.regenerate, currentMessageIndex]);

        // 如果是 reasoning 类型消息，使用特殊的渲染逻辑
        if (message.message_type === "reasoning") {
            const [isExpanded, setIsExpanded] = useState(false); // 默认折叠
            
            // 检查内容是否完整（基于是否以句号等结束符结尾判断）
            const isComplete = displayedContent.trim().endsWith('.') || 
                             displayedContent.trim().endsWith('。') || 
                             displayedContent.trim().endsWith('!') || 
                             displayedContent.trim().endsWith('！') ||
                             displayedContent.trim().endsWith('?') ||
                             displayedContent.trim().endsWith('？');

            const lines = displayedContent.split('\n');
            const previewLines = lines.slice(-2); // 默认显示最后2行

            return (
                <div className="my-2 p-3 bg-slate-50 border-l-4 border-indigo-500 rounded-r-lg max-w-[80%]">
                    <div className="flex items-center gap-2 mb-2">
                        <div className={`w-2 h-2 bg-indigo-500 rounded-full ${!isComplete ? 'animate-pulse' : ''}`}></div>
                        <span className="text-sm font-medium text-indigo-700">
                            {isComplete ? '思考完成' : '思考中...'}
                        </span>
                    </div>
                    <div className="text-sm text-gray-700 whitespace-pre-wrap font-mono">
                        {isExpanded ? displayedContent : (
                            <>
                                {lines.length > 2 && <div className="text-gray-400 text-xs mb-1">...</div>}
                                {previewLines.join('\n')}
                            </>
                        )}
                    </div>
                    {lines.length > 2 && (
                        <button
                            onClick={() => setIsExpanded(!isExpanded)}
                            className="mt-2 text-xs text-indigo-600 hover:text-indigo-800 underline cursor-pointer"
                        >
                            {isExpanded ? '收起' : '展开思考'}
                        </button>
                    )}
                </div>
            );
        }

        const handleCopy = useCallback(() => {
            writeText(displayedContent);
            setCopyIconState("ok");
        }, [displayedContent]);

        useEffect(() => {
            if (copyIconState === "ok") {
                const timer = setTimeout(() => {
                    setCopyIconState("copy");
                }, 1500);

                return () => clearTimeout(timer);
            }
        }, [copyIconState]);

        // 当 messageRegenerateLength 变化时更新选中的 index 为最新
        useEffect(() => {
            if (message.regenerate?.length) {
                handleMessageIndexChange(message.regenerate.length + 1);
            }
            // eslint-disable-next-line react-hooks/exhaustive-deps
        }, [message.regenerate?.length]);

        const handleMessageIndexChange = useCallback(
            (newMessageIndex: number) => {
                if (newMessageIndex < 1) {
                    newMessageIndex = 1;
                }
                if (newMessageIndex > message.regenerate.length + 1) {
                    newMessageIndex = message.regenerate.length + 1;
                }
                setCurrentMessageIndex(newMessageIndex);
            },
            [currentMessageIndex, message.regenerate],
        );

        // 自定义解析器来处理自定义标签（移除了 think 标签处理）
        const customParser = (
            markdown: string,
            customTags: { [key: string]: (match: RegExpExecArray) => string },
        ) => {
            let result = markdown;

            Object.keys(customTags).forEach((tag) => {
                // 匹配完整的标签对
                const completeRegex = new RegExp(
                    `<${tag}([^>]*)>([\\s\\S]*?)<\\/${tag}>`,
                    "g",
                );
                let match;
                while ((match = completeRegex.exec(markdown)) !== null) {
                    const replacement = customTags[tag](match);
                    result = result.replace(match[0], replacement);
                }
            });

            return result;
        };

        const customTags = {
            fileattachment: (match: RegExpExecArray) =>
                `\n<fileattachment ${match[1]}></fileattachment>\n`,
            bangwebtomarkdown: (match: RegExpExecArray) =>
                `\n<bangwebtomarkdown ${match[1]}></bangwebtomarkdown>\n`,
            bangweb: (match: RegExpExecArray) =>
                `\n<bangweb ${match[1]}></bangweb>\n`,
        };

        const markdownContent = useMemo(
            () => customParser(displayedContent, customTags),
            [displayedContent],
        );

        // 定义允许的自定义标签及其属性，结合默认 schema
        const sanitizeSchema = {
            ...defaultSchema,
            tagNames: [
                ...(defaultSchema.tagNames || []),
                "fileattachment",
                "bangwebtomarkdown",
                "bangweb",
            ],
            attributes: {
                ...(defaultSchema.attributes || {}),
                fileattachment: [
                    ...(defaultSchema.attributes?.fileattachment || []),
                    "attachment_id",
                    "attachment_url",
                    "attachment_type",
                    "attachment_content",
                ],
                bangwebtomarkdown: [
                    ...(defaultSchema.attributes?.bangwebtomarkdown || []),
                ],
                bangweb: [
                    ...(defaultSchema.attributes?.bangweb || []),
                ],
            },
        };

        const markdownElement = useMemo(
            () => (
                <ReactMarkdown
                    children={markdownContent}
                    remarkPlugins={[
                        remarkMath,
                        remarkBreaks,
                        remarkCustomCompenent,
                    ]}
                    rehypePlugins={[
                        rehypeRaw,
                        [rehypeSanitize, sanitizeSchema],
                        rehypeKatex,
                        rehypeHighlight,
                    ]}
                    components={{
                        code: ({ className, children }) => {
                            const match = /language-(\w+)/.exec(
                                className || "",
                            );
                            return match ? (
                                <CodeBlock
                                    language={match[1]}
                                    onCodeRun={onCodeRun}
                                >
                                    {children}
                                </CodeBlock>
                            ) : (
                                <code
                                    className={className}
                                    style={{
                                        overflow: "auto",
                                    }}
                                >
                                    {children}
                                </code>
                            );
                        },
                        fileattachment: MessageFileAttachment,
                        bangwebtomarkdown: MessageWebContent,
                        bangweb: MessageWebContent,
                        tipscomponent: TipsComponent,
                    } as CustomComponents}
                />
            ),
            [markdownContent, onCodeRun],
        );

        return (
            <div
                className={
                    "group relative py-4 px-5 rounded-2xl inline-block max-w-[65%] transition-all duration-200 " +
                    (message.message_type === "user"
                        ? "self-end bg-secondary text-primary"
                        : "self-start bg-background text-foreground border border-border")
                }
            >
                {message.regenerate?.length > 0 ? (
                    <div className="mb-2 flex flex-row justify-end items-center text-gray-500 font-medium text-xs">
                        <span
                            className="cursor-pointer mx-2 py-1.5 px-3 rounded-lg transition-all duration-200 hover:bg-gray-100"
                            onClick={() =>
                                handleMessageIndexChange(
                                    currentMessageIndex - 1,
                                )
                            }
                        >
                            {"<"}
                        </span>
                        <span>
                            {currentMessageIndex} /{" "}
                            {message.regenerate?.length + 1}
                        </span>
                        <span
                            className="cursor-pointer mx-2 py-1.5 px-3 rounded-lg transition-all duration-200 hover:bg-gray-100"
                            onClick={() =>
                                handleMessageIndexChange(
                                    currentMessageIndex + 1,
                                )
                            }
                        >
                            {">"}
                        </span>
                    </div>
                ) : null}

                <div className="prose prose-sm max-w-none">{markdownElement}</div>
                {message.attachment_list?.filter(
                    (a: any) => a.attachment_type === "Image",
                ).length ? (
                    <div
                        className="w-[300px] flex flex-col"
                    >
                        {message.attachment_list
                            .filter((a: any) => a.attachment_type === "Image")
                            .map((attachment: any) => {
                                console.log(attachment);
                                return attachment;
                            })
                            .map((attachment: any) => (
                                <img
                                    key={attachment.attachment_url}
                                    className="flex-1"
                                    src={attachment.attachment_content}
                                />
                            ))}
                    </div>
                ) : null}

                <div className={`hidden group-hover:flex items-center absolute -bottom-9 py-3 px-4 box-border h-10 rounded-[21px] border border-border bg-background ${message.message_type === "user" ? "right-0" : "left-0"}`}>
                    {(message.message_type === "assistant" || message.message_type === "response") ? (
                        <IconButton
                            icon={<Refresh fill="black" />}
                            onClick={onMessageRegenerate}
                        />
                    ) : null}
                    <IconButton
                        icon={
                            copyIconState === "copy" ? (
                                <Copy fill="black" />
                            ) : (
                                <Ok fill="black" />
                            )
                        }
                        onClick={handleCopy}
                    />
                </div>
            </div>
        );
    },
);

export default MessageItem;
