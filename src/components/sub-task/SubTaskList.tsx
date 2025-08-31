import React, { useState, useMemo, useCallback } from "react";
import { SubTaskExecutionSummary } from "../../data/SubTask";
import { useSubTaskManager } from "../../hooks/useSubTaskManager";
import { useSubTaskEvents } from "../../hooks/useSubTaskEvents";
import SubTaskItem from "./SubTaskItem";
import { Button } from "../ui/button";
import { ChevronLeft, ChevronRight } from "lucide-react";

export interface SubTaskListProps {
    conversation_id: number;
    message_id?: number;
    className?: string;
    onTaskDetailView?: (execution: SubTaskExecutionSummary) => void;
}

const ITEMS_PER_PAGE = 5;

const SubTaskList: React.FC<SubTaskListProps> = ({
    conversation_id,
    message_id,
    className = "",
    onTaskDetailView,
}) => {
    const [currentPage, setCurrentPage] = useState(0);

    // Use sub-task management hooks
    const {
        executions,
        error,
        refresh,
        hasRunningTasks,
    } = useSubTaskManager({
        conversation_id,
        message_id,
    });

    // Listen to real-time events
    useSubTaskEvents({
        conversation_id,
        onStatusUpdate: () => {
            // Refresh data when status updates
            refresh();
        },
        onTaskCompleted: () => {
            refresh();
        },
        onTaskFailed: () => {
            refresh();
        },
    });

    // Handle task detail view
    const handleViewDetail = useCallback(
        (execution: SubTaskExecutionSummary) => {
            if (onTaskDetailView) {
                onTaskDetailView(execution);
            }
        },
        [onTaskDetailView]
    );

    // Pagination calculations
    const { totalPages, currentPageItems, hasPrevPage, hasNextPage } = useMemo(() => {
        const sortedExecutions = [...executions].sort((a, b) => {
            // Running tasks first, then by creation time (newest first)
            if (a.status === "running" && b.status !== "running") return -1;
            if (b.status === "running" && a.status !== "running") return 1;
            return new Date(b.created_time).getTime() - new Date(a.created_time).getTime();
        });

        const totalPages = Math.ceil(sortedExecutions.length / ITEMS_PER_PAGE);
        const startIndex = currentPage * ITEMS_PER_PAGE;
        const currentPageItems = sortedExecutions.slice(startIndex, startIndex + ITEMS_PER_PAGE);

        return {
            totalPages,
            currentPageItems,
            hasPrevPage: currentPage > 0,
            hasNextPage: currentPage < totalPages - 1,
        };
    }, [executions, currentPage]);

    // Reset current page when executions change
    React.useEffect(() => {
        if (currentPage >= totalPages && totalPages > 0) {
            setCurrentPage(totalPages - 1);
        }
    }, [totalPages, currentPage]);

    // Don't render if no sub-tasks
    if (executions.length === 0) {
        return null;
    }

    const handlePrevPage = () => {
        if (hasPrevPage) {
            setCurrentPage(currentPage - 1);
        }
    };

    const handleNextPage = () => {
        if (hasNextPage) {
            setCurrentPage(currentPage + 1);
        }
    };

    return (
        <div className={`border-border flex justify-center p-3 ${className}`}>
            <div className="flex items-center justify-between">
                {/* Sub-task items */}
                <div className="flex items-center gap-2 flex-1">
                    {currentPageItems.map((execution) => (
                        <SubTaskItem
                            key={execution.id}
                            execution={execution}
                            onViewDetail={handleViewDetail}
                        />
                    ))}
                </div>

                {/* Pagination controls - only show if needed */}
                {totalPages > 1 && (
                    <div className="flex items-center gap-1 ml-4">
                        <Button
                            variant="ghost"
                            size="sm"
                            onClick={handlePrevPage}
                            disabled={!hasPrevPage}
                            className="h-6 w-6 p-0"
                        >
                            <ChevronLeft className="w-3 h-3" />
                        </Button>

                        <span className="text-xs text-muted-foreground px-1">
                            {currentPage + 1} / {totalPages}
                        </span>

                        <Button
                            variant="ghost"
                            size="sm"
                            onClick={handleNextPage}
                            disabled={!hasNextPage}
                            className="h-6 w-6 p-0"
                        >
                            <ChevronRight className="w-3 h-3" />
                        </Button>
                    </div>
                )}
            </div>
        </div>
    );
};

export default React.memo(SubTaskList);