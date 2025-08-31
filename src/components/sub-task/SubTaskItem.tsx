import React from "react";
import { SubTaskExecutionSummary } from "../../data/SubTask";
import { getStatusIcon } from "../../services/subTaskService";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "../ui/tooltip";

export interface SubTaskItemProps {
    execution: SubTaskExecutionSummary;
    onViewDetail?: (execution: SubTaskExecutionSummary) => void;
}

const SubTaskItem: React.FC<SubTaskItemProps> = ({
    execution,
    onViewDetail,
}) => {
    const handleClick = () => {
        if (onViewDetail) {
            onViewDetail(execution);
        }
    };

    return (
        <TooltipProvider>
            <Tooltip>
                <TooltipTrigger asChild>
                    <div
                        className={"inline-flex items-center justify-center w-8 h-8 rounded-full bg-white border-2 border-black cursor-pointer" + (execution.status === "running" ? " w-12 h-12" : "")}
                        onClick={handleClick}
                    >
                        {/* Status icon - black icon on white background */}
                        <div className="text-black">
                            {getStatusIcon(execution.status)}
                        </div>
                    </div>
                </TooltipTrigger>

                <TooltipContent side="bottom" className="max-w-xs">
                    <div className="space-y-1 text-sm">
                        <div className="font-medium">{execution.task_name}</div>
                        <div className="text-muted-foreground text-xs">
                            状态: {execution.status}
                        </div>
                        {execution.task_prompt && (
                            <div className="text-muted-foreground text-xs line-clamp-2">
                                提示: {execution.task_prompt}
                            </div>
                        )}
                        <div className="text-muted-foreground text-xs">
                            创建时间: {execution.created_time.toLocaleTimeString()}
                        </div>
                        {execution.token_count > 0 && (
                            <div className="text-muted-foreground text-xs">
                                Token: {execution.token_count}
                            </div>
                        )}
                    </div>
                </TooltipContent>
            </Tooltip>
        </TooltipProvider>
    );
};

export default React.memo(SubTaskItem);