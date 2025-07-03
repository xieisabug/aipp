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
                    "message-item " +
                    (message.message_type === "user"
                        ? "user-message"
                        : "bot-message")
                }
            >
                {message.regenerate?.length > 0 ? (
                    <div className="message-regenerate-bar">
                        <span
                            className="message-regenerate-bar-button"
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
                            className="message-regenerate-bar-button"
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
                                            className="llm-thinking-badge"
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
                {message.attachment_list.filter(
                    (a: any) => a.attachment_type === "Image",
                ).length ? (
                    <div
                        className="message-image"
                        style={{
                            width: "300px",
                            display: "flex",
                            flexDirection: "column",
                        }}
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
                                    style={{ flex: 1 }}
                                    src={attachment.attachment_content}
                                />
                            ))}
                    </div>
                ) : null}

                <div className="message-item-button-container">
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
