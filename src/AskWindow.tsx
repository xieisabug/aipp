import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { listen, once, emitTo } from "@tauri-apps/api/event";
import ReactMarkdown, { Components } from "react-markdown";
import remarkMath from "remark-math";
import rehypeRaw from "rehype-raw";
import rehypeKatex from "rehype-katex";
import { toast } from "sonner";

import Copy from "./assets/copy.svg?react";
import Ok from "./assets/ok.svg?react";
import OpenFullUI from "./assets/open-fullui.svg?react";
import Setting from "./assets/setting.svg?react";
import Add from "./assets/add.svg?react";
import AskWindowPrepare from "./components/AskWindowPrepare";
import AskAIHint from "./components/AskAIHint";
import IconButton from "./components/IconButton";
import { throttle } from "lodash";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import CodeBlock from "./components/CodeBlock";
import useFileManagement from "./hooks/useFileManagement";
import InputArea from "./components/conversation/InputArea";
import { useConversationEvents } from "./hooks/useConversationEvents";
import { StreamEvent } from "./data/Conversation";
const appWindow = getCurrentWebviewWindow();

interface AiResponse {
    conversation_id: number;
}
interface CustomComponents extends Components {
    antthinking: React.ElementType;
}

function AskWindow() {
    const [query, setQuery] = useState<string>("");
    const [response, setResponse] = useState<string>("");
    const [messageId, setMessageId] = useState<number>(-1);
    const inputRef = useRef<HTMLTextAreaElement>(null);
    const [aiIsResponsing, setAiIsResponsing] = useState<boolean>(false);
    const [copySuccess, setCopySuccess] = useState<boolean>(false);
    const [selectedText, setSelectedText] = useState<string>("");
    // 当前对话 id，用于在 ChatUIWindow 中自动选中
    const [conversationId, setConversationId] = useState<string>("");
    // 独立的错误状态管理
    const [errorMessage, setErrorMessage] = useState<string>("");

    // 清除错误信息
    const clearError = useCallback(() => {
        setErrorMessage("");
    }, []);

    // 错误处理回调
    const handleError = useCallback((errorMessage: string) => {
        console.error("Stream error in AskWindow:", errorMessage);
        setAiIsResponsing(false);
        // 设置错误信息，而不是替换响应内容
        setErrorMessage(errorMessage);
    }, []);

    // 使用共享的消息事件处理 hook
    const { streamingMessages } = useConversationEvents({
        conversationId: conversationId,
        onMessageUpdate: (streamEvent: StreamEvent) => {
            // 处理错误消息类型
            if (streamEvent.message_type === "error") {
                setErrorMessage(streamEvent.content);
                setAiIsResponsing(false);
                return;
            }
            
            // 更新正常响应内容
            if (!streamEvent.is_done) {
                setResponse(streamEvent.content);
                setMessageId(streamEvent.message_id);
            }
        },
        onAiResponseComplete: () => {
            setAiIsResponsing(false);
        },
        onError: handleError,
    });

    useEffect(() => {
        invoke<string>("get_selected_text_api").then((text) => {
            console.log("get_selected_text_api", text);
            setSelectedText(text);
        });

        listen<string>("get_selected_text_event", (event) => {
            console.log("get_selected_text_event", event.payload);
            setSelectedText(event.payload);
        });

        // 监听错误通知事件
        const unsubscribe = listen("conversation-window-error-notification", (event) => {
            const errorMsg = event.payload as string;
            console.error("Received error notification in AskWindow:", errorMsg);
            
            // 显示错误通知
            toast.error(`AI请求失败: ${errorMsg}`);
            
            // 重置AI响应状态
            setAiIsResponsing(false);
            
            // 设置错误信息，而不是替换响应内容
            setErrorMessage(errorMsg);
        });

        return () => {
            if (unsubscribe) {
                unsubscribe.then((f) => f());
            }
        };
    }, []);

    const handleSubmit = () => {
        if (aiIsResponsing) {
            return;
        }
        setAiIsResponsing(true);
        setResponse("");
        setErrorMessage(""); // 清除之前的错误信息
        
        invoke<AiResponse>("ask_ai", {
            request: {
                prompt: query,
                conversation_id: conversationId,
                assistant_id: 1,
                attachment_list: fileInfoList?.map((i) => i.id),
            },
        })
        .then((res) => {
            // 记录新的 conversationId，便于后续在 ChatUIWindow 中定位
            if (
                res.conversation_id !== undefined &&
                res.conversation_id !== null
            ) {
                setConversationId(res.conversation_id.toString());
                console.log(
                    "AskWindow 获取到 conversation_id",
                    res.conversation_id,
                );
            }

            console.log("ask ai response", res);
            // 事件处理现在由共享的 useConversationEvents hook 管理
        })
        .catch((error) => {
            console.error("Ask AI request failed:", error);
            setAiIsResponsing(false);
            
            // 显示错误信息
            const errorMsg = typeof error === 'string' ? error : 'Unknown error occurred';
            setErrorMessage(errorMsg);
            
            // 显示错误通知
            toast.error(`发送消息失败: ${errorMsg}`);
        });
    };

    const onSend = throttle(() => {
        if (aiIsResponsing) {
            console.log("Cancelling AI");
            invoke("cancel_ai", { conversationId: +conversationId })
                .then(() => {
                    setAiIsResponsing(false);
                })
                .catch((error) => {
                    console.error("Cancel AI failed:", error);
                    setAiIsResponsing(false);
                    toast.error("取消请求失败");
                });
        } else {
            console.log("Sending query to AI");
            handleSubmit();
        }
    }, 200);

    useEffect(() => {
        const handleShortcut = async (event: KeyboardEvent) => {
            if (event.key === "Escape") {
                console.log("Closing window");
                await appWindow.hide();
            } else if (event.key === "i" && event.ctrlKey) {
                await openChatUI();
                await appWindow.hide();
            }
        };

        if (inputRef.current) {
            inputRef.current.focus();
        }

        window.addEventListener("keydown", handleShortcut);

        return () => {
            window.removeEventListener("keydown", handleShortcut);
            // 清理逻辑现在由 useConversationEvents hook 处理
        };
    }, []);

    const openConfig = async () => {
        await invoke("open_config_window");
    };

    const openChatUI = async () => {
        const sendSelect = () => {
            if (!conversationId) {
                console.warn(
                    "AskWindow：当前 conversationId 为空，无法自动选中对话",
                );
                return;
            }
            emitTo("chat_ui", "select_conversation", conversationId);
        };

        // 注册一次性监听，防止窗口尚未加载完成时事件丢失
        once("chat-ui-window-load", () => {
            sendSelect();
        });

        // 尝试立即发送一次，以覆盖已打开窗口的场景
        sendSelect();

        // 打开 / 显示 Chat UI 窗口
        await invoke("open_chat_ui_window");
    };

    const handleArtifact = useCallback((lang: string, inputStr: string) => {
        invoke("run_artifacts", { lang, inputStr }).then((res) => {
            console.log(res);
        });
    }, []);

    const startNewConversation = () => {
        setQuery("");
        setResponse("");
        setMessageId(-1);
        setAiIsResponsing(false);
        setConversationId("");
        setErrorMessage(""); // 清除错误信息
    };

    const { fileInfoList, handleChooseFile, handleDeleteFile, handlePaste } =
        useFileManagement();

    // 合并响应显示（支持流式和最终响应）
    const displayResponse = useMemo(() => {
        if (messageId !== -1 && streamingMessages.has(messageId)) {
            return streamingMessages.get(messageId)?.content || response;
        }
        return response;
    }, [messageId, streamingMessages, response]);

    return (
        <div className="flex justify-center items-center h-screen">
            <div
                className="bg-white shadow-lg w-full h-screen"
                data-tauri-drag-region
            >
                <InputArea
                    inputText={query}
                    setInputText={setQuery}
                    fileInfoList={fileInfoList}
                    handleChooseFile={handleChooseFile}
                    handleDeleteFile={handleDeleteFile}
                    handlePaste={handlePaste}
                    handleSend={onSend}
                    aiIsResponsing={aiIsResponsing}
                    placement="top"
                />
                <div className="prose prose-sm p-5 pb-16 max-w-none bg-white">
                    {/* 错误信息显示区域 */}
                    {errorMessage && (
                        <div className="mb-4 bg-red-50 border border-red-200 rounded-lg p-4">
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
                                    <div className="text-sm text-red-700">
                                        {errorMessage}
                                    </div>
                                </div>
                                <button
                                    onClick={clearError}
                                    className="flex-shrink-0 text-red-400 hover:text-red-600 transition-colors"
                                    title="清除错误信息"
                                >
                                    <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                                        <path fillRule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clipRule="evenodd" />
                                    </svg>
                                </button>
                            </div>
                        </div>
                    )}

                    {/* 正常内容显示区域 */}
                    {messageId !== -1 ? (
                        response == "" ? (
                            <AskAIHint />
                        ) : (
                            <ReactMarkdown
                                children={displayResponse}
                                remarkPlugins={[remarkMath]}
                                rehypePlugins={[rehypeRaw, rehypeKatex]}
                                components={
                                    {
                                        code({
                                            node,
                                            className,
                                            children,
                                            ref,
                                            ...props
                                        }) {
                                            const match = /language-(\w+)/.exec(
                                                className || "",
                                            );
                                            return match ? (
                                                <CodeBlock
                                                    language={match[1]}
                                                    onCodeRun={handleArtifact}
                                                >
                                                    {String(children).replace(
                                                        /\n$/,
                                                        "",
                                                    )}
                                                </CodeBlock>
                                            ) : (
                                                <code
                                                    {...props}
                                                    ref={ref}
                                                    className={className}
                                                >
                                                    {children}
                                                </code>
                                            );
                                        },
                                        antthinking({ children }) {
                                            return (
                                                <div>
                                                    <div
                                                        className="bg-blue-100 text-blue-800 px-2 py-1 rounded text-sm font-medium inline-block"
                                                        title={children}
                                                        data-thinking={children}
                                                    >
                                                        思考...
                                                    </div>
                                                </div>
                                            );
                                        },
                                    } as CustomComponents
                                }
                            />
                        )
                    ) : (
                        <AskWindowPrepare selectedText={selectedText} />
                    )}
                </div>
                <div
                    className="w-full h-8 fixed bottom-0 left-0 flex items-center justify-end pr-2.5 bg-gray-100"
                    data-tauri-drag-region
                >
                    {messageId !== -1 && !aiIsResponsing && (
                        <IconButton
                            icon={<Add fill="black" />}
                            onClick={startNewConversation}
                        />
                    )}
                    {messageId !== -1 && !aiIsResponsing ? (
                        <IconButton
                            icon={
                                copySuccess ? (
                                    <Ok fill="black" />
                                ) : (
                                    <Copy fill="black" />
                                )
                            }
                            onClick={() => {
                                writeText(displayResponse);
                                setCopySuccess(true);
                                setTimeout(() => {
                                    setCopySuccess(false);
                                }, 1500);
                            }}
                        />
                    ) : null}

                    <IconButton
                        icon={<OpenFullUI fill="black" />}
                        onClick={openChatUI}
                    />
                    <IconButton
                        icon={<Setting fill="black" />}
                        onClick={openConfig}
                    />
                </div>
            </div>
        </div>
    );
}

export default AskWindow;
