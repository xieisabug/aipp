import React, { useCallback, useEffect, useMemo, useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { open } from "@tauri-apps/plugin-shell";
import ReactMarkdown, { Components } from "react-markdown";
import remarkMath from "remark-math";
import remarkBreaks from "remark-breaks";
import remarkGfm from "remark-gfm";
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
import { Edit2 } from "lucide-react";
import CodeBlock from "./CodeBlock";
import MessageFileAttachment from "./MessageFileAttachment";
import MessageWebContent from "./conversation/MessageWebContent";
import ReasoningMessage from "./ReasoningMessage";
import { Message, StreamEvent } from "../data/Conversation";
import {
    usePerformanceMonitor,
    measureSync,
} from "../hooks/usePerformanceMonitor";
import { ShineBorder } from "./magicui/shine-border";

interface CustomComponents extends Components {
    fileattachment: React.ElementType;
    bangwebtomarkdown: React.ElementType;
    bangweb: React.ElementType;
}

// 将常量移到组件外部避免重复创建
const CUSTOM_TAGS = {
    fileattachment: (match: RegExpExecArray) =>
        `\n<fileattachment ${match[1]}></fileattachment>\n`,
    bangwebtomarkdown: (match: RegExpExecArray) =>
        `\n<bangwebtomarkdown ${match[1]}></bangwebtomarkdown>\n`,
    bangweb: (match: RegExpExecArray) => `\n<bangweb ${match[1]}></bangweb>\n`,
};

// 定义允许的自定义标签及其属性，结合默认 schema
const SANITIZE_SCHEMA = {
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
        bangweb: [...(defaultSchema.attributes?.bangweb || [])],
    },
};

// ReactMarkdown 插件配置
const REMARK_PLUGINS = [
    remarkMath,
    remarkBreaks,
    remarkGfm,
    remarkCustomCompenent,
] as const;

const REHYPE_PLUGINS = [
    rehypeRaw,
    [rehypeSanitize, SANITIZE_SCHEMA] as const,
    rehypeKatex,
    rehypeHighlight,
] as const;

// ReactMarkdown 组件配置的基础部分
const MARKDOWN_COMPONENTS_BASE = {
    fileattachment: MessageFileAttachment,
    bangwebtomarkdown: MessageWebContent,
    bangweb: MessageWebContent,
    tipscomponent: TipsComponent,
} as const;

interface MessageItemProps {
    message: Message;
    streamEvent?: StreamEvent;
    onCodeRun?: (lang: string, code: string) => void;
    onMessageRegenerate?: () => void;
    onMessageEdit?: () => void;
    // Reasoning 展开状态相关 props
    isReasoningExpanded?: boolean;
    onToggleReasoningExpand?: () => void;
    // ShineBorder 动画状态
    shouldShowShineBorder?: boolean;
}

const MessageItem = React.memo(
    ({
        message,
        streamEvent,
        onCodeRun,
        onMessageRegenerate,
        onMessageEdit,
        isReasoningExpanded = false,
        onToggleReasoningExpand,
        shouldShowShineBorder = false,
    }: MessageItemProps) => {
        // 性能监控
        usePerformanceMonitor(
            "MessageItem",
            [
                message.id,
                message.content,
                message.message_type,
                streamEvent?.is_done,
                isReasoningExpanded,
            ],
            false,
        );
        const [copyIconState, setCopyIconState] = useState<"copy" | "ok">(
            "copy",
        );

        // 使用新的版本管理逻辑
        const displayedContent = useMemo(() => {
            return message.content;
        }, [message.content]);

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

        // 如果是 reasoning 类型消息，使用专门的组件渲染
        if (message.message_type === "reasoning") {
            return (
                <ReasoningMessage
                    message={message}
                    streamEvent={streamEvent}
                    displayedContent={displayedContent}
                    isReasoningExpanded={isReasoningExpanded}
                    onToggleReasoningExpand={onToggleReasoningExpand}
                />
            );
        }

        // 如果是错误类型消息，使用专门的错误渲染
        if (message.message_type === "error") {
            return (
                <div className="group relative py-4 px-5 rounded-2xl inline-block max-w-[65%] transition-all duration-200 self-start bg-red-50 text-red-800 border border-red-200">
                    <div className="flex items-start space-x-3">
                        <div className="flex-shrink-0 w-5 h-5 mt-0.5">
                            <svg className="w-5 h-5 text-red-500" fill="currentColor" viewBox="0 0 20 20">
                                <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clipRule="evenodd" />
                            </svg>
                        </div>
                        <div className="flex-1">
                            <div className="text-sm font-medium text-red-800 mb-1">
                                AI Request Failed
                            </div>
                            <div className="prose prose-sm max-w-none text-red-700">
                                {displayedContent}
                            </div>
                        </div>
                    </div>
                    <div
                        className={`hidden group-hover:flex items-center absolute -bottom-9 py-3 px-4 box-border h-10 rounded-[21px] border border-red-200 bg-red-50 left-0`}
                    >
                        <IconButton
                            icon={
                                copyIconState === "copy" ? (
                                    <Copy fill="#dc2626" />
                                ) : (
                                    <Ok fill="#dc2626" />
                                )
                            }
                            onClick={handleCopy}
                        />
                    </div>
                </div>
            );
        }

        // 自定义解析器来处理自定义标签（移除了 think 标签处理）
        const customParser = useCallback(
            (
                markdown: string,
                customTags: {
                    [key: string]: (match: RegExpExecArray) => string;
                },
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
            },
            [],
        );

        const markdownContent = useMemo(
            () =>
                measureSync(
                    `markdown-parsing-${message.id}`,
                    () => customParser(displayedContent, CUSTOM_TAGS),
                    false,
                ),
            [displayedContent, customParser],
        );

        // 使用 useMemo 缓存 markdown 组件配置
        const markdownComponents = useMemo(
            () =>
                ({
                    ...MARKDOWN_COMPONENTS_BASE,
                    code: ({
                        className,
                        children,
                    }: {
                        className?: string;
                        children: React.ReactNode;
                    }) => {
                        const match = /language-(\w+)/.exec(className || "");
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
                    a: ({
                        href,
                        children,
                        ...props
                    }: {
                        href?: string;
                        children: React.ReactNode;
                        [key: string]: any;
                    }) => {
                        const handleClick = useCallback(
                            (e: React.MouseEvent) => {
                                e.preventDefault();
                                if (href) {
                                    open(href).catch(console.error);
                                }
                            },
                            [href],
                        );

                        return (
                            <a
                                href={href}
                                onClick={handleClick}
                                className="text-blue-600 hover:text-blue-800 underline cursor-pointer"
                                {...props}
                            >
                                {children}
                            </a>
                        );
                    },
                }) as CustomComponents,
            [onCodeRun],
        );

        const markdownElement = useMemo(
            () =>
                measureSync(
                    `markdown-render-${message.id}`,
                    () => (
                        <ReactMarkdown
                            children={markdownContent}
                            remarkPlugins={REMARK_PLUGINS as any}
                            rehypePlugins={REHYPE_PLUGINS as any}
                            components={markdownComponents}
                        />
                    ),
                    false,
                ),
            [markdownContent, markdownComponents],
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
                {shouldShowShineBorder && (
                    <ShineBorder
                        shineColor={["#A07CFE", "#FE8FB5", "#FFBE7B"]}
                        borderWidth={2}
                        duration={8}
                    />
                )}
                <div className="prose prose-sm max-w-none">
                    {markdownElement}
                </div>
                {message.attachment_list?.filter(
                    (a: any) => a.attachment_type === "Image",
                ).length ? (
                    <div className="w-[300px] flex flex-col">
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

                <div
                    className={`hidden group-hover:flex items-center absolute -bottom-9 py-3 px-4 box-border h-10 rounded-[21px] border border-border bg-background ${message.message_type === "user" ? "right-0" : "left-0"}`}
                >
                    {(message.message_type === "assistant" ||
                        message.message_type === "response" ||
                        message.message_type === "user") &&
                    onMessageEdit ? (
                        <IconButton
                            icon={<Edit2 size={16} color="black" />}
                            onClick={onMessageEdit}
                        />
                    ) : null}
                    {(message.message_type === "assistant" ||
                        message.message_type === "response" ||
                        message.message_type === "user") &&
                    onMessageRegenerate ? (
                        <IconButton
                            icon={<Refresh fill="black" />}
                            onClick={onMessageRegenerate}
                        />
                    ) : null}
                    {/* Error messages only show copy button, no edit/regenerate */}
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
    // 自定义比较函数，只在关键属性变化时才重新渲染
    (prevProps, nextProps) => {
        // 基本消息属性比较
        if (prevProps.message.id !== nextProps.message.id) return false;
        if (prevProps.message.content !== nextProps.message.content)
            return false;
        if (prevProps.message.message_type !== nextProps.message.message_type)
            return false;

        // regenerate 数组比较
        const prevRegenerate = prevProps.message.regenerate;
        const nextRegenerate = nextProps.message.regenerate;
        if (prevRegenerate?.length !== nextRegenerate?.length) return false;

        // 流式事件比较
        const prevStreamEvent = prevProps.streamEvent;
        const nextStreamEvent = nextProps.streamEvent;
        if (prevStreamEvent?.is_done !== nextStreamEvent?.is_done) return false;
        if (prevStreamEvent?.content !== nextStreamEvent?.content) return false;

        // reasoning 展开状态比较
        if (prevProps.isReasoningExpanded !== nextProps.isReasoningExpanded)
            return false;

        // ShineBorder 动画状态比较
        if (prevProps.shouldShowShineBorder !== nextProps.shouldShowShineBorder)
            return false;

        // 回调函数比较（通常应该是稳定的）
        if (prevProps.onCodeRun !== nextProps.onCodeRun) return false;
        if (prevProps.onMessageRegenerate !== nextProps.onMessageRegenerate)
            return false;
        if (prevProps.onMessageEdit !== nextProps.onMessageEdit) return false;
        if (
            prevProps.onToggleReasoningExpand !==
            nextProps.onToggleReasoningExpand
        )
            return false;

        return true; // 所有关键属性都相同，不需要重新渲染
    },
);

export default MessageItem;
