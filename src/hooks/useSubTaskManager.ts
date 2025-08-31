import { useCallback, useEffect, useState } from "react";
import {
    SubTaskDefinition,
    SubTaskExecutionSummary,
    SubTaskExecutionDetail,
    CreateSubTaskRequest,
    ListSubTaskDefinitionsParams,
    ListSubTaskExecutionsParams,
    UseSubTaskManagerOptions,
} from "../data/SubTask";
import { subTaskService } from "../services/subTaskService";

export function useSubTaskManager(options: UseSubTaskManagerOptions) {
    const [definitions, setDefinitions] = useState<SubTaskDefinition[]>([]);
    const [executions, setExecutions] = useState<SubTaskExecutionSummary[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // 加载任务定义
    const loadDefinitions = useCallback(async (params?: ListSubTaskDefinitionsParams) => {
        try {
            setLoading(true);
            const result = await subTaskService.listDefinitions(params);
            setDefinitions(result);
            setError(null);
        } catch (err) {
            setError(err instanceof Error ? err.message : "加载任务定义失败");
        } finally {
            setLoading(false);
        }
    }, []);

    // 加载执行记录
    const loadExecutions = useCallback(
        async (params?: Partial<ListSubTaskExecutionsParams>) => {
            try {
                setLoading(true);
                const fullParams: ListSubTaskExecutionsParams = {
                    parent_conversation_id: options.conversation_id,
                    parent_message_id: options.message_id,
                    source_id: options.source_id,
                    ...params,
                };
                const result = await subTaskService.listExecutions(fullParams);
                setExecutions(result);
                setError(null);
            } catch (err) {
                setError(err instanceof Error ? err.message : "加载执行记录失败");
            } finally {
                setLoading(false);
            }
        },
        [options.conversation_id, options.message_id, options.source_id]
    );

    // 创建子任务
    const createSubTask = useCallback(
        async (request: Omit<CreateSubTaskRequest, "parent_conversation_id" | "parent_message_id" | "source_id">) => {
            try {
                setLoading(true);
                const fullRequest: CreateSubTaskRequest = {
                    ...request,
                    parent_conversation_id: options.conversation_id,
                    parent_message_id: options.message_id,
                    source_id: options.source_id || 0,
                };
                const execution_id = await subTaskService.createExecution(fullRequest);

                // 刷新执行记录
                await loadExecutions();
                setError(null);
                return execution_id;
            } catch (err) {
                const errorMessage = err instanceof Error ? err.message : "创建子任务失败";
                setError(errorMessage);
                throw new Error(errorMessage);
            } finally {
                setLoading(false);
            }
        },
        [options.conversation_id, options.message_id, options.source_id, loadExecutions]
    );

    // 取消任务
    const cancelSubTask = useCallback(
        async (execution_id: number) => {
            if (!options.source_id) {
                throw new Error("缺少 source_id");
            }

            try {
                setLoading(true);
                await subTaskService.cancelExecution(execution_id, options.source_id);

                // 刷新执行记录
                await loadExecutions();
                setError(null);
            } catch (err) {
                const errorMessage = err instanceof Error ? err.message : "取消任务失败";
                setError(errorMessage);
                throw new Error(errorMessage);
            } finally {
                setLoading(false);
            }
        },
        [options.source_id, loadExecutions]
    );

    // 获取任务详情
    const getExecutionDetail = useCallback(
        async (execution_id: number): Promise<SubTaskExecutionDetail | null> => {
            if (!options.source_id) {
                throw new Error("缺少 source_id");
            }

            try {
                const result = await subTaskService.getExecutionDetail(execution_id, options.source_id);
                return result;
            } catch (err) {
                const errorMessage = err instanceof Error ? err.message : "获取任务详情失败";
                setError(errorMessage);
                throw new Error(errorMessage);
            }
        },
        [options.source_id]
    );

    // 刷新数据
    const refresh = useCallback(async () => {
        await Promise.all([loadDefinitions({ source_id: options.source_id }), loadExecutions()]);
    }, [loadDefinitions, loadExecutions, options.source_id]);

    // 初始化加载
    useEffect(() => {
        refresh();
    }, [refresh]);

    return {
        // 数据
        definitions,
        executions,
        loading,
        error,

        // 操作方法
        loadDefinitions,
        loadExecutions,
        createSubTask,
        cancelSubTask,
        getExecutionDetail,
        refresh,

        // 清除错误
        clearError: useCallback(() => setError(null), []),

        // 计算属性
        runningTasks: executions.filter((exec) => exec.status === "running"),
        completedTasks: executions.filter((exec) => exec.status === "success"),
        failedTasks: executions.filter((exec) => exec.status === "failed"),
        hasRunningTasks: executions.some((exec) => exec.status === "running"),

        // 统计信息
        stats: {
            total: executions.length,
            pending: executions.filter((exec) => exec.status === "pending").length,
            running: executions.filter((exec) => exec.status === "running").length,
            success: executions.filter((exec) => exec.status === "success").length,
            failed: executions.filter((exec) => exec.status === "failed").length,
            cancelled: executions.filter((exec) => exec.status === "cancelled").length,
        },
    };
}

export default useSubTaskManager;
