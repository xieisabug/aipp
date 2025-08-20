import React, { useState, useCallback, useMemo, useEffect } from "react";
import { Play, Loader2, CheckCircle, XCircle, Blocks, ChevronDown, ChevronUp, RotateCcw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ShineBorder } from "@/components/magicui/shine-border";
import { DEFAULT_SHINE_BORDER_CONFIG } from "@/utils/shineConfig";
import { invoke } from "@tauri-apps/api/core";
import { MCPToolCall } from "@/data/MCPToolCall";
import { MCPToolCallUpdateEvent } from "@/data/Conversation";

interface McpToolCallProps {
    serverName?: string;
    toolName?: string;
    parameters?: string;
    conversationId?: number;
    messageId?: number;
    callId?: number; // If provided, this is an existing call
    mcpToolCallStates?: Map<number, MCPToolCallUpdateEvent>; // Global MCP states
}

type ExecutionState = "idle" | "pending" | "executing" | "success" | "failed";

const JsonDisplay: React.FC<{ content: string; maxHeight?: string; className?: string }> = ({
    content,
    maxHeight = "120px",
    className = "",
}) => {
    const formattedJson = useMemo(() => {
        try {
            const parsed = JSON.parse(content);
            return JSON.stringify(parsed, null, 2);
        } catch {
            return content;
        }
    }, [content]);

    return (
        <div className={`border rounded ${className}`} style={{ maxHeight: maxHeight }}>
            <ScrollArea>
                <pre className="text-xs font-mono p-2 whitespace-pre-wrap break-words mt-0 mb-0">{formattedJson}</pre>
            </ScrollArea>
        </div>
    );
};

const StatusIndicator: React.FC<{ state: ExecutionState }> = ({ state }) => {
    switch (state) {
        case "idle":
            return null;
        case "pending":
            return (
                <Badge variant="secondary" className="flex items-center gap-1 ml-3">
                    <Loader2 className="h-3 w-3 animate-spin" />
                    等待中
                </Badge>
            );
        case "executing":
            return (
                <Badge variant="secondary" className="flex items-center gap-1 ml-3">
                    <Loader2 className="h-3 w-3 animate-spin" />
                    执行中
                </Badge>
            );
        case "success":
            return (
                <Badge
                    variant="default"
                    className="flex items-center gap-1 bg-green-100 text-green-800 border-green-200 ml-3"
                >
                    <CheckCircle className="h-3 w-3" />
                    成功
                </Badge>
            );
        case "failed":
            return (
                <Badge variant="destructive" className="flex items-center gap-1 ml-3">
                    <XCircle className="h-3 w-3" />
                    失败
                </Badge>
            );
        default:
            return null;
    }
};

const McpToolCall: React.FC<McpToolCallProps> = ({
    serverName = "未知服务器",
    toolName = "未知工具",
    parameters = "{}",
    conversationId,
    messageId,
    callId,
    mcpToolCallStates,
}) => {
    const [executionState, setExecutionState] = useState<ExecutionState>("idle");
    const [executionResult, setExecutionResult] = useState<string | null>(null);
    const [executionError, setExecutionError] = useState<string | null>(null);
    const [isExpanded, setIsExpanded] = useState<boolean>(false);
    const [toolCallId, setToolCallId] = useState<number | null>(callId || null);
    // 移除前端自动执行，避免与后端 detect_and_process_mcp_calls 的自动执行叠加

    // 监听全局MCP状态变化
    useEffect(() => {
        if (mcpToolCallStates && toolCallId && mcpToolCallStates.has(toolCallId)) {
            const globalState = mcpToolCallStates.get(toolCallId)!;
            console.log(`McpToolCall ${toolCallId} received global state update:`, globalState);

            // 同步全局状态到本地状态
            switch (globalState.status) {
                case "pending":
                    setExecutionState("pending");
                    break;
                case "executing":
                    setExecutionState("executing");
                    break;
                case "success":
                    setExecutionState("success");
                    setExecutionResult(globalState.result || null);
                    setExecutionError(null);
                    break;
                case "failed":
                    setExecutionState("failed");
                    setExecutionError(globalState.error || null);
                    setExecutionResult(null);
                    break;
            }
        }
    }, [mcpToolCallStates, toolCallId]);

    // 检查执行状态
    const isFailed = executionState === "failed";
    const isExecuting = executionState === "executing";
    const canExecute = executionState === "idle" || executionState === "failed"; // 失败状态也可以重新执行
    const isRunning = executionState === "executing" || executionState === "pending"; // 运行状态用于显示闪亮边框

    // 如果提供了 callId，尝试获取已有的执行结果
    useEffect(() => {
        if (callId && executionState === "idle") {
            const fetchExistingResult = async () => {
                try {
                    const result = await invoke<MCPToolCall>("get_mcp_tool_call", {
                        callId: callId,
                    });

                    if (result.status === "success" && result.result) {
                        setExecutionResult(result.result);
                        setExecutionState("success");
                    } else if (result.status === "failed" && result.error) {
                        setExecutionError(result.error);
                        setExecutionState("failed");
                    }
                } catch (error) {
                    console.warn("Failed to fetch existing tool call result:", error);
                }
            };

            fetchExistingResult();
        }
    }, [callId, executionState]);

    // 如果没有 callId，尝试根据消息参数查询是否存在相关的工具调用记录
    useEffect(() => {
        if (!callId && conversationId && messageId && executionState === "idle") {
            const findExistingToolCall = async () => {
                try {
                    const allCalls = await invoke<MCPToolCall[]>("get_mcp_tool_calls_by_conversation", {
                        conversationId: conversationId,
                    });

                    // 查找匹配的工具调用（相同的消息ID、服务器名和工具名）
                    const matchingCall = allCalls.find(
                        (call) =>
                            call.message_id === messageId &&
                            call.server_name === serverName &&
                            call.tool_name === toolName &&
                            call.parameters === parameters
                    );

                    if (matchingCall) {
                        setToolCallId(matchingCall.id);

                        if (matchingCall.status === "success" && matchingCall.result) {
                            setExecutionResult(matchingCall.result);
                            setExecutionState("success");
                        } else if (matchingCall.status === "failed" && matchingCall.error) {
                            setExecutionError(matchingCall.error);
                            setExecutionState("failed");
                        } else if (matchingCall.status === "executing") {
                            setExecutionState("executing");
                        }
                    }
                } catch (error) {
                    console.warn("Failed to find existing tool call:", error);
                }
            };

            findExistingToolCall();
        }
    }, [callId, conversationId, messageId, serverName, toolName, parameters, executionState]);

    // 注意：后端 `detect_and_process_mcp_calls` 已根据助手配置自动执行，这里不再做自动执行

    const handleExecute = useCallback(async () => {
        if (!conversationId) {
            console.error("conversation_id is required for execution");
            return;
        }

        try {
            setExecutionState("executing");
            setExecutionResult(null);
            setExecutionError(null);

            let currentCallId = toolCallId;

            // Create tool call if it doesn't exist
            if (!currentCallId) {
                const createdCall = await invoke<MCPToolCall>("create_mcp_tool_call", {
                    conversationId: conversationId,
                    messageId: messageId,
                    serverName: serverName,
                    toolName: toolName,
                    parameters,
                });
                currentCallId = createdCall.id;
                setToolCallId(currentCallId);
            }

            // Execute the tool call
            const result = await invoke<MCPToolCall>("execute_mcp_tool_call", {
                callId: currentCallId,
            });

            if (result.status === "success" && result.result) {
                setExecutionResult(result.result);
                setExecutionState("success");
            } else if (result.status === "failed" && result.error) {
                setExecutionError(result.error);
                setExecutionState("failed");
            }
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : "执行失败";
            setExecutionError(errorMessage);
            setExecutionState("failed");
        }
    }, [conversationId, messageId, serverName, toolName, parameters, toolCallId]);

    const renderResult = (fixedHeight = false) => {
        if (executionResult) {
            return (
                <div className="mt-2">
                    <span className="text-xs text-muted-foreground">结果:</span>
                    <div className="border rounded mt-1">
                        <ScrollArea className="h-72">
                            <pre className="whitespace-pre-wrap break-words mt-0 mb-0">{executionResult}</pre>
                        </ScrollArea>
                    </div>
                </div>
            );
        }

        if (executionError) {
            return (
                <div className="mt-2">
                    <span className="text-xs text-muted-foreground">错误:</span>
                    <div
                        className="border rounded mt-1"
                        style={{ height: fixedHeight ? "200px" : "auto", maxHeight: fixedHeight ? "none" : "200px" }}
                    >
                        <ScrollArea className="h-full w-full">
                            <div className="text-xs font-mono bg-muted p-2">
                                <div className="text-red-600 whitespace-pre-wrap break-words">
                                    <strong>错误:</strong> {executionError}
                                </div>
                            </div>
                        </ScrollArea>
                    </div>
                </div>
            );
        }

        return null;
    };

    return (
        <div className="w-full max-w-[600px] my-1 p-2 border border-border rounded-md bg-card overflow-hidden relative">
            {isRunning && (
                <ShineBorder
                    shineColor={DEFAULT_SHINE_BORDER_CONFIG.shineColor}
                    borderWidth={DEFAULT_SHINE_BORDER_CONFIG.borderWidth}
                    duration={DEFAULT_SHINE_BORDER_CONFIG.duration}
                />
            )}
            <div className="flex items-center justify-between">
                <div className="flex items-center gap-2 text-sm min-w-0 flex-1">
                    <Blocks className="h-4 w-4 flex-shrink-0" />
                    <span className="truncate">{serverName}</span>
                    <span className="text-xs font-bold text-muted-foreground flex-shrink-0"> - </span>
                    <span className="truncate">{toolName}</span>
                </div>
                <div className="flex items-center gap-1 flex-shrink-0">
                    <StatusIndicator state={executionState} />
                    {!isExpanded && canExecute && (
                        <Button
                            onClick={handleExecute}
                            disabled={isExecuting}
                            size="sm"
                            variant="ghost"
                            className="h-7 w-7 p-0 flex-shrink-0"
                            title={isFailed ? "重新执行" : "执行"}
                        >
                            {isExecuting ? (
                                <Loader2 className="h-3 w-3 animate-spin" />
                            ) : isFailed ? (
                                <RotateCcw className="h-3 w-3" />
                            ) : (
                                <Play className="h-3 w-3" />
                            )}
                        </Button>
                    )}
                    <Button
                        onClick={() => setIsExpanded(!isExpanded)}
                        size="sm"
                        variant="ghost"
                        className="h-7 w-7 p-0 flex-shrink-0"
                    >
                        {isExpanded ? <ChevronUp className="h-3 w-3" /> : <ChevronDown className="h-3 w-3" />}
                    </Button>
                </div>
            </div>

            {isExpanded && (
                <div className="mt-2 space-y-2 max-w-full overflow-hidden">
                    <div className="max-w-full overflow-hidden">
                        <span className="text-xs font-medium mb-1 text-muted-foreground">参数:</span>
                        <JsonDisplay content={parameters} maxHeight="120px" className="mt-1" />
                    </div>
                    {canExecute && (
                        <div className="flex items-center gap-2">
                            <Button
                                onClick={handleExecute}
                                disabled={isExecuting}
                                size="sm"
                                className="flex items-center gap-1 h-7 text-xs"
                            >
                                {isExecuting ? (
                                    <Loader2 className="h-3 w-3 animate-spin" />
                                ) : isFailed ? (
                                    <RotateCcw className="h-3 w-3" />
                                ) : (
                                    <Play className="h-3 w-3" />
                                )}
                                {isFailed ? "重新执行" : "执行"}
                            </Button>
                        </div>
                    )}
                    <div className="max-w-full overflow-hidden">{renderResult()}</div>
                </div>
            )}
        </div>
    );
};

export default McpToolCall;
