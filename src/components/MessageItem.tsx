import React, { useCallback, useEffect, useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import ReactMarkdown, { Components } from "react-markdown";
import remarkMath from "remark-math";
import remarkBreaks from "remark-breaks";
import rehypeRaw from "rehype-raw";
import rehypeKatex from "rehype-katex";
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
    think: React.ElementType;
    fileattachment: React.ElementType;
    bangwebtomarkdown: React.ElementType;
    bangweb: React.ElementType;
}

const MessageItem = React.memo(
    ({ message, onCodeRun, onMessageRegenerate }: any) => {
        const [copyIconState, setCopyIconState] = useState<"copy" | "ok">(
            "copy",
        );
        const [currentMessageContent, setCurrentMessageContent] =
            useState<string>(
                message.regenerate?.length > 0
                    ? message.regenerate[message.regenerate.length - 1].content
                    : message.content,
            );
        const [currentMessageIndex, setCurrentMessageIndex] = useState<number>(
            message.regenerate?.length > 0 ? message.regenerate.length + 1 : -1,
        );

        const handleCopy = useCallback(() => {
            writeText(currentMessageContent);
            setCopyIconState("ok");
        }, [currentMessageContent]);

        useEffect(() => {
            if (copyIconState === "ok") {
                const timer = setTimeout(() => {
                    setCopyIconState("copy");
                }, 1500);

                return () => clearTimeout(timer);
            }
        }, [copyIconState]);

        // 处理message content变化
        useEffect(() => {
            let index =
                message.regenerate?.length > 0
                    ? message.regenerate.length + 1
                    : -1;
            setCurrentMessageIndex(index);
            if (message.regenerate?.length > 0) {
                if (index === 1) {
                    setCurrentMessageContent(message.content);
                } else {
                    setCurrentMessageContent(
                        message.regenerate[index - 2].content,
                    );
                }
            } else {
                setCurrentMessageContent(message.content);
            }
        }, [message]);

        // 处理regenerate的时候，自动选中最新的message
        const messageRegenerateLength = message.regenerate?.length ?? 0;
        useEffect(() => {
            if (messageRegenerateLength !== 0) {
                handleMessageIndexChange(message.regenerate.length + 1);
            }
        }, [messageRegenerateLength]);

        const handleMessageIndexChange = useCallback(
            (newMessageIndex: number) => {
                if (newMessageIndex < 1) {
                    newMessageIndex = 1;
                }
                if (newMessageIndex > message.regenerate.length + 1) {
                    newMessageIndex = message.regenerate.length + 1;
                }
                setCurrentMessageIndex(newMessageIndex);
                if (newMessageIndex === 1) {
                    setCurrentMessageContent(message.content);
                } else {
                    setCurrentMessageContent(
                        message.regenerate[newMessageIndex - 2].content,
                    );
                }
            },
            [currentMessageIndex, message.regenerate],
        );

        // 自定义解析器来处理自定义标签
        const customParser = (
            markdown: string,
            customTags: { [key: string]: (match: RegExpExecArray) => string },
        ) => {
            let result = markdown;

            Object.keys(customTags).forEach((tag) => {
                const regex = new RegExp(
                    `<${tag}([^>]*)>([\\s\\S]*?)<\/${tag}>`,
                    "g",
                );
                let match;
                while ((match = regex.exec(markdown)) !== null) {
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

        return (
            <div
                className={
                    "group relative py-4 px-5 rounded-2xl inline-block max-w-[65%] leading-6 transition-all duration-200 " +
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

                <div className="prose prose-sm max-w-none [&>p]:m-0 [&>p]:text-sm">
                    <ReactMarkdown
                        children={customParser(currentMessageContent, customTags)}
                        remarkPlugins={[
                            remarkMath,
                            remarkBreaks,
                            remarkCustomCompenent,
                        ]}
                        rehypePlugins={[rehypeRaw, rehypeKatex]}
                        components={
                            {
                                code: ({ className, children }) => {
                                    const match = /language-(\w+)/.exec(
                                        className || "",
                                    );
                                    return match ? (
                                        <CodeBlock
                                            language={match[1]}
                                            onCodeRun={onCodeRun}
                                        >
                                            {String(children).replace(/\n$/, "")}
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
                                think: ({ children }) => {
                                    return (
                                        <div>
                                            <div
                                                className="py-2 px-4 bg-gradient-to-r from-indigo-500 to-purple-600 text-white rounded-xl inline-block cursor-pointer text-xs font-medium transition-all duration-200 shadow-md hover:-translate-y-0.5 hover:shadow-lg"
                                                title={children}
                                                data-thinking={children}
                                            >
                                                思考...
                                            </div>
                                        </div>
                                    );
                                },
                                fileattachment: MessageFileAttachment,
                                bangwebtomarkdown: MessageWebContent,
                                bangweb: MessageWebContent,
                                tipscomponent: TipsComponent,
                            } as CustomComponents
                        }
                    />
                </div>
                {message.attachment_list.filter(
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
                    {message.message_type === "assistant" ? (
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
