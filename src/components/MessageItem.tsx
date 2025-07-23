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
import { Message, StreamEvent } from "../data/Conversation";

interface CustomComponents extends Components {
    fileattachment: React.ElementType;
    bangwebtomarkdown: React.ElementType;
    bangweb: React.ElementType;
}

interface MessageItemProps {
    message: Message;
    streamEvent?: StreamEvent;
    onCodeRun?: (lang: string, code: string) => void;
    onMessageRegenerate?: () => void;
}

const MessageItem = React.memo(
    ({ message, streamEvent, onCodeRun, onMessageRegenerate }: MessageItemProps) => {
        const [copyIconState, setCopyIconState] = useState<"copy" | "ok">(
            "copy",
        );
        const [currentMessageIndex, setCurrentMessageIndex] = useState<number>(
            message.regenerate && message.regenerate.length > 0 ? message.regenerate.length + 1 : -1,
        );
        // 仅记录当前查看的版本索引，真实内容按需计算，避免在流式更新时重复 setState
        const displayedContent = useMemo(() => {
            if (message.regenerate && message.regenerate.length > 0) {
                // currentMessageIndex 为 1 表示原始回复
                if (currentMessageIndex === 1) return message.content;
                // 其它值指向 regenerate 数组（index 从 0 开始）
                return message.regenerate[currentMessageIndex - 2]?.content ?? message.content;
            }
            return message.content;
        }, [message.content, message.regenerate, currentMessageIndex]);

        // 如果是 reasoning 类型消息，使用特殊的渲染逻辑
        if (message.message_type === "reasoning") {
            const [isExpanded, setIsExpanded] = useState(false);
            const [currentTime, setCurrentTime] = useState(new Date());
            
            // 使用 start_time 和 finish_time 来判断思考状态，也考虑 streamEvent 的状态
            const isComplete = message.finish_time !== null || (streamEvent?.is_done === true);
            const isThinking = message.start_time !== null && !isComplete;

            // 为正在思考的消息添加定时器，实时更新显示时间
            useEffect(() => {
                if (isThinking) {
                    const timer = setInterval(() => {
                        setCurrentTime(new Date());
                    }, 1000); // 每秒更新一次
                    
                    return () => clearInterval(timer);
                }
            }, [isThinking]);

            // 计算思考时间 - 统一使用后端时间基准
            const calculateThinkingTime = () => {
                // 优先使用 streamEvent 中后端提供的精确时间信息
                if (streamEvent?.duration_ms !== undefined) {
                    const seconds = Math.floor(streamEvent.duration_ms / 1000);
                    if (seconds < 60) return `${seconds}秒`;
                    const minutes = Math.floor(seconds / 60);
                    const remainingSeconds = seconds % 60;
                    return `${minutes}分${remainingSeconds}秒`;
                }
                
                // 如果有后端提供的结束时间，使用后端时间计算
                if (message.start_time && message.finish_time) {
                    const startTime = new Date(message.start_time);
                    const endTime = new Date(message.finish_time);
                    const diffMs = endTime.getTime() - startTime.getTime();
                    const seconds = Math.floor(diffMs / 1000);
                    if (seconds < 60) return `${seconds}秒`;
                    const minutes = Math.floor(seconds / 60);
                    const remainingSeconds = seconds % 60;
                    return `${minutes}分${remainingSeconds}秒`;
                }

                // 正在思考时：基于后端开始时间和当前时间计算实时时间
                if (message.start_time && !message.finish_time) {
                    const startTime = new Date(message.start_time);
                    // 使用定时器更新的 currentTime 来保证实时性
                    const diffMs = Math.max(0, currentTime.getTime() - startTime.getTime());
                    const seconds = Math.floor(diffMs / 1000);
                    if (seconds < 60) return `${seconds}秒`;
                    const minutes = Math.floor(seconds / 60);
                    const remainingSeconds = seconds % 60;
                    return `${minutes}分${remainingSeconds}秒`;
                }

                return '';
            };

            // 格式化状态文本
            const formatStatusText = (baseText: string) => {
                const timeStr = calculateThinkingTime();
                return timeStr ? `${baseText}(${baseText === '思考中...' ? '已' : ''}思考 ${timeStr})` : baseText;
            };

            const lines = displayedContent.split('\n');
            const previewLines = lines.slice(-3); // 思考中时显示最后3行

            // 思考完成时的小模块展示
            if (isComplete && !isExpanded) {
                return (
                    <div 
                        className="my-2 p-2 bg-gray-50 border-l-4 border-gray-400 rounded-r-lg w-80 max-w-[60%] cursor-pointer hover:bg-gray-100 transition-colors"
                        onClick={() => setIsExpanded(true)}
                    >
                        <div className="flex items-center gap-2">
                            <div className="w-2 h-2 bg-gray-500 rounded-full"></div>
                            <span className="text-sm font-medium text-gray-700">
                                {formatStatusText('思考完成')}
                            </span>
                            <span className="text-xs text-gray-400 ml-auto">点击展开</span>
                        </div>
                    </div>
                );
            }

            // 完整展示（思考完成展开或思考中）
            return (
                <div className="my-2 p-3 bg-gray-50 border-l-4 border-gray-400 rounded-r-lg max-w-[80%]">
                    <div className="flex items-center gap-2 mb-2">
                        <div className={`w-2 h-2 bg-gray-500 rounded-full ${isThinking ? 'animate-pulse' : ''}`}></div>
                        <span className="text-sm font-medium text-gray-700">
                            {formatStatusText(isComplete ? '思考完成' : '思考中...')}
                        </span>
                    </div>
                    <div className="text-sm text-gray-600 whitespace-pre-wrap font-mono">
                        {isThinking && lines.length > 3 ? (
                            <>
                                <div className="text-gray-400 text-xs mb-1">...</div>
                                {previewLines.join('\n')}
                            </>
                        ) : (
                            displayedContent
                        )}
                    </div>
                    {/* 思考中时的展开按钮 */}
                    {isThinking && lines.length > 3 && (
                        <button
                            onClick={() => setIsExpanded(true)}
                            className="mt-2 text-xs text-gray-600 hover:text-gray-800 underline cursor-pointer"
                        >
                            展开思考
                        </button>
                    )}
                    {/* 思考完成时的收起按钮 */}
                    {isComplete && (
                        <button
                            onClick={() => setIsExpanded(false)}
                            className="mt-2 text-xs text-gray-600 hover:text-gray-800 underline cursor-pointer"
                        >
                            收起
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
            if (message.regenerate && message.regenerate.length > 0) {
                handleMessageIndexChange(message.regenerate.length + 1);
            }
            // eslint-disable-next-line react-hooks/exhaustive-deps
        }, [message.regenerate?.length]);

        const handleMessageIndexChange = useCallback(
            (newMessageIndex: number) => {
                if (newMessageIndex < 1) {
                    newMessageIndex = 1;
                }
                const maxIndex = message.regenerate ? message.regenerate.length + 1 : 1;
                if (newMessageIndex > maxIndex) {
                    newMessageIndex = maxIndex;
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
                                    onCodeRun={onCodeRun || (() => {})}
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
                {message.regenerate && message.regenerate.length > 0 ? (
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
                            {message.regenerate.length + 1}
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
                    {(message.message_type === "assistant" || message.message_type === "response") && onMessageRegenerate ? (
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
