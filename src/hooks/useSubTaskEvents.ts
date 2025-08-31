import { useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { SubTaskStatusUpdateEvent, UseSubTaskEventsOptions, SubTaskExecutionDetail } from "../data/SubTask";

export function useSubTaskEvents(options: UseSubTaskEventsOptions) {
    const { conversation_id, onStatusUpdate, onTaskCompleted, onTaskFailed } = options;

    // 使用 ref 来避免闭包问题
    const onStatusUpdateRef = useRef(onStatusUpdate);
    const onTaskCompletedRef = useRef(onTaskCompleted);
    const onTaskFailedRef = useRef(onTaskFailed);

    // 更新 ref
    useEffect(() => {
        onStatusUpdateRef.current = onStatusUpdate;
        onTaskCompletedRef.current = onTaskCompleted;
        onTaskFailedRef.current = onTaskFailed;
    }, [onStatusUpdate, onTaskCompleted, onTaskFailed]);

    // 处理状态更新事件
    const handleStatusUpdate = useCallback((event: SubTaskStatusUpdateEvent) => {
        // 调用通用状态更新回调
        if (onStatusUpdateRef.current) {
            onStatusUpdateRef.current(event);
        }

        // 根据状态调用特定回调
        if (event.status === "success" && onTaskCompletedRef.current) {
            const detail: SubTaskExecutionDetail = {
                id: event.execution_id,
                task_code: event.task_code,
                task_name: event.task_name,
                task_prompt: "", // 这里需要从其他地方获取
                status: event.status,
                created_time: new Date(), // 这里需要从其他地方获取
                token_count: event.token_count || 0,
                result_content: event.result_content,
                error_message: event.error_message,
                llm_model_name: undefined,
                input_token_count: 0,
                output_token_count: 0,
                started_time: event.started_time,
                finished_time: event.finished_time,
            };
            onTaskCompletedRef.current(detail);
        }

        if (event.status === "failed" && onTaskFailedRef.current) {
            const detail: SubTaskExecutionDetail = {
                id: event.execution_id,
                task_code: event.task_code,
                task_name: event.task_name,
                task_prompt: "", // 这里需要从其他地方获取
                status: event.status,
                created_time: new Date(), // 这里需要从其他地方获取
                token_count: event.token_count || 0,
                result_content: event.result_content,
                error_message: event.error_message,
                llm_model_name: undefined,
                input_token_count: 0,
                output_token_count: 0,
                started_time: event.started_time,
                finished_time: event.finished_time,
            };
            onTaskFailedRef.current(detail);
        }
    }, []);

    useEffect(() => {
        // 监听子任务状态更新事件
        const eventName = `sub_task_update_${conversation_id}`;

        const unlisten = listen<SubTaskStatusUpdateEvent>(eventName, (event) => {
            handleStatusUpdate(event.payload);
        });

        // 清理监听器
        return () => {
            unlisten.then((unlistenFn) => unlistenFn());
        };
    }, [conversation_id, handleStatusUpdate]);

    return {
        // 这个 hook 主要是处理事件，不返回数据
        // 如果需要的话可以添加状态管理
    };
}

export default useSubTaskEvents;
