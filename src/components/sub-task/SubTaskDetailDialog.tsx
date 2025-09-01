import React, { useState, useEffect } from "react";
import { SubTaskExecutionDetail, SubTaskExecutionSummary } from "../../data/SubTask";
import {
    subTaskService,
    getStatusColor,
    getStatusIcon,
    getStatusText,
    formatTokenCount,
    formatDuration,
} from "../../services/subTaskService";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "../ui/dialog";
import { Button } from "../ui/button";
import { Badge } from "../ui/badge";
import { ScrollArea } from "../ui/scroll-area";
import { StopCircle, RefreshCw, Clock, Zap, AlertCircle } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/card";

export interface SubTaskDetailDialogProps {
    isOpen: boolean;
    onClose: () => void;
    execution: SubTaskExecutionSummary;
    onCancel?: (execution_id: number) => void;
}

const SubTaskDetailDialog: React.FC<SubTaskDetailDialogProps> = ({ isOpen, onClose, execution, onCancel }) => {
    const [detail, setDetail] = useState<SubTaskExecutionDetail | null>(null);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // Load detailed information when dialog opens
    useEffect(() => {
        if (isOpen && execution) {
            loadDetail();
        }
    }, [isOpen, execution.id]);

    const loadDetail = async () => {
        try {
            setLoading(true);
            setError(null);
            // 使用UI专用的详情获取方法（不需要鉴权）
            const detailData = await subTaskService.getExecutionDetailForUI(execution.id);
            setDetail(detailData);
        } catch (err) {
            setError(err instanceof Error ? err.message : "加载详情失败");
        } finally {
            setLoading(false);
        }
    };

    const handleCancel = async () => {
        if (onCancel && execution.status === "running") {
            try {
                // 取消任务操作交给父组件处理，因为父组件知道合适的source_id
                await onCancel(execution.id);
                onClose();
            } catch (error) {
                console.error("Failed to cancel task:", error);
            }
        }
    };

    const canCancel = execution.status === "running" && onCancel;

    return (
        <Dialog open={isOpen} onOpenChange={onClose}>
            <DialogContent className="max-w-2xl max-h-[80vh] flex flex-col">
                <DialogHeader>
                    <DialogTitle className="flex items-center gap-2">
                        <div className="text-lg flex items-center">{getStatusIcon(execution.status)}</div>
                        <span>{execution.task_name}</span>
                        <Badge className={getStatusColor(execution.status)}>{getStatusText(execution.status)}</Badge>
                    </DialogTitle>
                </DialogHeader>

                <div className="flex-1">
                    {loading ? (
                        <div className="flex items-center justify-center h-32">
                            <RefreshCw className="w-6 h-6 animate-spin" />
                            <span className="ml-2">加载中...</span>
                        </div>
                    ) : error ? (
                        <div className="flex items-center justify-center h-32 text-destructive">
                            <AlertCircle className="w-6 h-6" />
                            <span className="ml-2">{error}</span>
                        </div>
                    ) : (
                        <ScrollArea className="h-[400px]">
                            <div className="space-y-4">
                                {/* Basic Information */}
                                <Card>
                                    <CardHeader className="pb-3">
                                        <CardTitle className="text-sm">基本信息</CardTitle>
                                    </CardHeader>
                                    <CardContent className="space-y-2">
                                        <div className="grid grid-cols-2 gap-4 text-sm">
                                            <div>
                                                <span className="text-muted-foreground">任务代码:</span>
                                                <span className="ml-2 font-mono">{execution.task_code}</span>
                                            </div>
                                            <div>
                                                <span className="text-muted-foreground">创建时间:</span>
                                                <span className="ml-2">{execution.created_time.toLocaleString()}</span>
                                            </div>
                                            {detail?.started_time && (
                                                <div>
                                                    <span className="text-muted-foreground">开始时间:</span>
                                                    <span className="ml-2">{detail.started_time.toLocaleString()}</span>
                                                </div>
                                            )}
                                            {detail?.finished_time && (
                                                <div>
                                                    <span className="text-muted-foreground">完成时间:</span>
                                                    <span className="ml-2">
                                                        {detail.finished_time.toLocaleString()}
                                                    </span>
                                                </div>
                                            )}
                                        </div>

                                        {/* Duration and Token Information */}
                                        <div className="flex items-center gap-4 pt-2">
                                            <div className="flex items-center gap-1 text-sm">
                                                <Clock className="w-4 h-4 text-muted-foreground" />
                                                <span>
                                                    耗时: {formatDuration(detail?.started_time, detail?.finished_time)}
                                                </span>
                                            </div>
                                            <div className="flex items-center gap-1 text-sm">
                                                <Zap className="w-4 h-4 text-muted-foreground" />
                                                <span>Tokens: {formatTokenCount(execution.token_count)}</span>
                                            </div>
                                        </div>
                                    </CardContent>
                                </Card>

                                {/* Task Prompt */}
                                {execution.task_prompt && (
                                    <Card>
                                        <CardHeader className="pb-3">
                                            <CardTitle className="text-sm">任务提示</CardTitle>
                                        </CardHeader>
                                        <CardContent>
                                            <div className="bg-muted p-3 rounded text-sm whitespace-pre-wrap">
                                                {execution.task_prompt}
                                            </div>
                                        </CardContent>
                                    </Card>
                                )}

                                {/* Result Content */}
                                {detail?.result_content && (
                                    <Card>
                                        <CardHeader className="pb-3">
                                            <CardTitle className="text-sm">执行结果</CardTitle>
                                        </CardHeader>
                                        <CardContent>
                                            <div className="bg-muted p-3 rounded text-sm whitespace-pre-wrap max-h-64 overflow-auto">
                                                {detail.result_content}
                                            </div>
                                        </CardContent>
                                    </Card>
                                )}

                                {/* Error Message */}
                                {detail?.error_message && (
                                    <Card className="border-destructive">
                                        <CardHeader className="pb-3">
                                            <CardTitle className="text-sm text-destructive">错误信息</CardTitle>
                                        </CardHeader>
                                        <CardContent>
                                            <div className="bg-destructive/10 p-3 rounded text-sm text-destructive whitespace-pre-wrap">
                                                {detail.error_message}
                                            </div>
                                        </CardContent>
                                    </Card>
                                )}

                                {/* Model Information */}
                                {detail?.llm_model_name && (
                                    <Card>
                                        <CardHeader className="pb-3">
                                            <CardTitle className="text-sm">模型信息</CardTitle>
                                        </CardHeader>
                                        <CardContent className="space-y-2">
                                            <div className="text-sm">
                                                <span className="text-muted-foreground">模型:</span>
                                                <span className="ml-2 font-mono">{detail.llm_model_name}</span>
                                            </div>
                                            <div className="grid grid-cols-2 gap-4 text-sm">
                                                <div>
                                                    <span className="text-muted-foreground">输入 Tokens:</span>
                                                    <span className="ml-2">
                                                        {formatTokenCount(detail.input_token_count)}
                                                    </span>
                                                </div>
                                                <div>
                                                    <span className="text-muted-foreground">输出 Tokens:</span>
                                                    <span className="ml-2">
                                                        {formatTokenCount(detail.output_token_count)}
                                                    </span>
                                                </div>
                                            </div>
                                        </CardContent>
                                    </Card>
                                )}
                            </div>
                        </ScrollArea>
                    )}
                </div>

                <DialogFooter className="flex justify-between">
                    <div>
                        {canCancel && (
                            <Button variant="destructive" onClick={handleCancel}>
                                <StopCircle className="w-4 h-4 mr-1" />
                                停止任务
                            </Button>
                        )}
                    </div>
                    <Button onClick={onClose}>关闭</Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
};

export default SubTaskDetailDialog;
