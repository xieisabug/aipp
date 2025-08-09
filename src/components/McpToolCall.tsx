import React, { useState, useCallback, useMemo } from "react";
import { Server, Wrench, Play, Maximize2, Loader2, CheckCircle, XCircle, Blocks } from "lucide-react";
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

const JsonDisplay: React.FC<{ content: string; maxHeight?: string }> = ({ content, maxHeight = "120px" }) => {
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
            <pre className="text-xs font-mono bg-secondary p-2 mt-1 mb-4 rounded text-secondary-foreground whitespace-pre-wrap break-words">
                {formattedJson}
            </pre>
        </ScrollArea>
    );
};

const StatusIndicator: React.FC<{ state: ExecutionState }> = ({ state }) => {
    switch (state) {
        case "idle": return null;
        case "executing":
            return <Badge variant="secondary" className="flex items-center gap-1"><Loader2 className="h-3 w-3 animate-spin" />执行中</Badge>;
        case "success":
            return <Badge variant="default" className="flex items-center gap-1 bg-green-100 text-green-800 border-green-200"><CheckCircle className="h-3 w-3" />成功</Badge>;
        case "error":
            return <Badge variant="destructive" className="flex items-center gap-1"><XCircle className="h-3 w-3" />错误</Badge>;
        default: return null;
    }
};

const McpToolCall: React.FC<McpToolCallProps> = ({
    server_name = "未知服务器",
    tool_name = "未知工具",
    parameters = "{}"
}) => {
    const [executionState, setExecutionState] = useState<ExecutionState>("idle");
    const [executionResult, setExecutionResult] = useState<ExecutionResult | null>(null);

    const handleExecute = useCallback(async () => {
        setExecutionState("executing");
        setExecutionResult(null);
        try {
            const result = await new Promise<ExecutionResult>((resolve) => {
                setTimeout(() => {
                    resolve({
                        success: true,
                        data: { message: "工具执行成功", timestamp: new Date().toISOString(), result: "模拟执行结果" }
                    });
                }, 2000);
            });
            setExecutionResult(result);
            setExecutionState(result.success ? "success" : "error");
        } catch (error) {
            const errorResult: ExecutionResult = {
                success: false,
                error: error instanceof Error ? error.message : "未知错误"
            };
            setExecutionResult(errorResult);
            setExecutionState("error");
        }
    }, [server_name, tool_name, parameters]);

    const renderResult = () => {
        if (!executionResult) return null;
        return (
            <div className="mt-2">
                <span className="text-xs text-muted-foreground">结果:</span>
                <ScrollArea className="w-full mt-1" style={{ maxHeight: "150px" }}>
                    <div className="text-xs font-mono bg-muted p-2 rounded">
                        {executionResult.success ? (
                            <pre className="whitespace-pre-wrap break-words text-muted-foreground">
                                {JSON.stringify(executionResult.data, null, 2)}
                            </pre>
                        ) : (
                            <div className="text-red-600"><strong>错误:</strong> {executionResult.error}</div>
                        )}
                    </div>
                </ScrollArea>
            </div>
        );
    };

    const DialogContent_: React.FC = () => (
        <DialogContent className="max-w-4xl max-h-[80vh]">
            <DialogHeader>
                <DialogTitle className="flex items-center gap-2">
                    <Blocks className="h-4 w-4" />
                    {server_name}
                    <span className="text-xs font-bold mb-1 text-muted-foreground"> - </span>
                    {tool_name}
                </DialogTitle>
            </DialogHeader>
            <div className="space-y-4">
                <div>
                    <h4 className="text-sm font-medium mb-2">参数:</h4>
                    <JsonDisplay content={parameters} maxHeight="400px" />
                </div>
                <div className="flex items-center gap-2">
                    <Button onClick={handleExecute} disabled={executionState === "executing"} size="sm" className="flex items-center gap-2">
                        {executionState === "executing" ? <Loader2 className="h-4 w-4 animate-spin" /> : <Play className="h-4 w-4" />}
                        执行
                    </Button>
                    <StatusIndicator state={executionState} />
                </div>
                {renderResult()}
            </div>
        </DialogContent>
    );

    return (
        <div className="w-full max-w-2xl my-1 p-2 border border-border rounded-md bg-card">
            <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-2 text-sm">
                    <Blocks className="h-4 w-4" />
                    {server_name}
                    <span className="text-xs font-bold mb-1 text-muted-foreground"> - </span>
                    {tool_name}
                </div>
                <StatusIndicator state={executionState} />
            </div>
            <div>
                <div>
                    <span className="text-xs font-medium mb-1 text-muted-foreground">参数:</span>
                    <JsonDisplay content={parameters} />
                </div>
                <div className="flex items-center gap-2">
                    <Button onClick={handleExecute} disabled={executionState === "executing"} size="sm" className="flex items-center gap-1 h-7 text-xs">
                        {executionState === "executing" ? <Loader2 className="h-3 w-3 animate-spin" /> : <Play className="h-3 w-3" />}
                        执行
                    </Button>
                    <Dialog>
                        <DialogTrigger asChild>
                            <Button variant="outline" size="sm" className="flex items-center gap-1 h-7 text-xs">
                                <Maximize2 className="h-3 w-3" />
                                展开
                            </Button>
                        </DialogTrigger>
                        <DialogContent_ />
                    </Dialog>
                </div>
                {renderResult()}
            </div>
        </div>
    );
};

export default McpToolCall;