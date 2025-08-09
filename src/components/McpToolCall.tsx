import React, { useState, useCallback, useMemo } from "react";
import { Server, Wrench, Play, Maximize2, Loader2, CheckCircle, XCircle } from "lucide-react";
import { Card, CardHeader, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";

interface McpToolCallProps {
    server_name?: string;
    tool_name?: string;
    parameters?: string;
}

type ExecutionState = "idle" | "executing" | "success" | "error";

interface ExecutionResult {
    success: boolean;
    data?: any;
    error?: string;
}

// JSON 格式化和语法高亮组件
const JsonDisplay: React.FC<{ content: string; maxHeight?: string }> = ({ content, maxHeight = "200px" }) => {
    const formattedJson = useMemo(() => {
        try {
            const parsed = JSON.parse(content);
            return JSON.stringify(parsed, null, 2);
        } catch {
            return content;
        }
    }, [content]);

    return (
        <ScrollArea className="w-full" style={{ maxHeight }}>
            <pre className="text-xs font-mono bg-secondary p-3 rounded-md text-secondary-foreground whitespace-pre-wrap break-words">
                {formattedJson}
            </pre>
        </ScrollArea>
    );
};

// 状态指示器组件
const StatusIndicator: React.FC<{ state: ExecutionState }> = ({ state }) => {
    switch (state) {
        case "idle":
            return null;
        case "executing":
            return (
                <Badge variant="secondary" className="flex items-center gap-1">
                    <Loader2 className="h-3 w-3 animate-spin" />
                    Executing
                </Badge>
            );
        case "success":
            return (
                <Badge variant="default" className="flex items-center gap-1 bg-green-100 text-green-800 border-green-200">
                    <CheckCircle className="h-3 w-3" />
                    Success
                </Badge>
            );
        case "error":
            return (
                <Badge variant="destructive" className="flex items-center gap-1">
                    <XCircle className="h-3 w-3" />
                    Error
                </Badge>
            );
        default:
            return null;
    }
};

const McpToolCall: React.FC<McpToolCallProps> = ({
    server_name = "Unknown Server",
    tool_name = "Unknown Tool",
    parameters = "{}"
}) => {
    const [executionState, setExecutionState] = useState<ExecutionState>("idle");
    const [executionResult, setExecutionResult] = useState<ExecutionResult | null>(null);

    // 执行工具调用
    const handleExecute = useCallback(async () => {
        setExecutionState("executing");
        setExecutionResult(null);

        try {
            // TODO: 这里需要调用实际的 MCP 工具执行 API
            // 现在先模拟执行过程
            const result = await new Promise<ExecutionResult>((resolve) => {
                setTimeout(() => {
                    resolve({
                        success: true,
                        data: {
                            message: "Tool executed successfully",
                            timestamp: new Date().toISOString(),
                            result: "Mock execution result"
                        }
                    });
                }, 2000);
            });

            setExecutionResult(result);
            setExecutionState(result.success ? "success" : "error");
        } catch (error) {
            const errorResult: ExecutionResult = {
                success: false,
                error: error instanceof Error ? error.message : "Unknown error occurred"
            };
            setExecutionResult(errorResult);
            setExecutionState("error");
        }
    }, [server_name, tool_name, parameters]);

    // 渲染执行结果
    const renderResult = () => {
        if (!executionResult) return null;

        return (
            <div className="mt-4">
                <div className="flex items-center gap-2 mb-2">
                    <span className="text-sm font-medium text-muted-foreground">Result:</span>
                    <StatusIndicator state={executionState} />
                </div>
                <ScrollArea className="w-full" style={{ maxHeight: "300px" }}>
                    <div className="text-xs font-mono bg-muted p-3 rounded-md">
                        {executionResult.success ? (
                            <pre className="whitespace-pre-wrap break-words text-muted-foreground">
                                {JSON.stringify(executionResult.data, null, 2)}
                            </pre>
                        ) : (
                            <div className="text-red-600">
                                <strong>Error:</strong> {executionResult.error}
                            </div>
                        )}
                    </div>
                </ScrollArea>
            </div>
        );
    };

    // 弹窗内容组件
    const DialogContent_: React.FC = () => (
        <DialogContent className="max-w-4xl max-h-[80vh]">
            <DialogHeader>
                <DialogTitle className="flex items-center gap-2">
                    <Server className="h-4 w-4" />
                    {server_name}
                    <Separator orientation="vertical" className="h-4" />
                    <Wrench className="h-4 w-4" />
                    {tool_name}
                </DialogTitle>
            </DialogHeader>
            <div className="space-y-4">
                <div>
                    <h4 className="text-sm font-medium mb-2">Parameters:</h4>
                    <JsonDisplay content={parameters} maxHeight="400px" />
                </div>
                <div className="flex items-center gap-2">
                    <Button 
                        onClick={handleExecute}
                        disabled={executionState === "executing"}
                        size="sm"
                        className="flex items-center gap-2"
                    >
                        {executionState === "executing" ? (
                            <Loader2 className="h-4 w-4 animate-spin" />
                        ) : (
                            <Play className="h-4 w-4" />
                        )}
                        Execute
                    </Button>
                    <StatusIndicator state={executionState} />
                </div>
                {renderResult()}
            </div>
        </DialogContent>
    );

    return (
        <Card className="w-full max-w-2xl my-2 border-border">
            <CardHeader className="pb-2">
                <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                        <div className="flex items-center gap-1 text-sm">
                            <Server className="h-4 w-4 text-muted-foreground" />
                            <span className="font-medium">{server_name}</span>
                        </div>
                        <Separator orientation="vertical" className="h-4" />
                        <div className="flex items-center gap-1 text-sm">
                            <Wrench className="h-4 w-4 text-muted-foreground" />
                            <span className="font-medium">{tool_name}</span>
                        </div>
                    </div>
                    <StatusIndicator state={executionState} />
                </div>
            </CardHeader>
            <CardContent className="space-y-3">
                {/* Parameters Section */}
                <div>
                    <h4 className="text-sm font-medium mb-2 text-muted-foreground">Parameters:</h4>
                    <JsonDisplay content={parameters} />
                </div>

                {/* Actions Section */}
                <div className="flex items-center gap-2 pt-2">
                    <Button 
                        onClick={handleExecute}
                        disabled={executionState === "executing"}
                        size="sm"
                        className="flex items-center gap-2"
                    >
                        {executionState === "executing" ? (
                            <Loader2 className="h-4 w-4 animate-spin" />
                        ) : (
                            <Play className="h-4 w-4" />
                        )}
                        Execute
                    </Button>
                    
                    <Dialog>
                        <DialogTrigger asChild>
                            <Button variant="outline" size="sm" className="flex items-center gap-2">
                                <Maximize2 className="h-4 w-4" />
                                Expand
                            </Button>
                        </DialogTrigger>
                        <DialogContent_ />
                    </Dialog>
                </div>

                {/* Result Section */}
                {renderResult()}
            </CardContent>
        </Card>
    );
};

export default McpToolCall;