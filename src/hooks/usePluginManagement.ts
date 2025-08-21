import { useState, useEffect, useCallback } from "react";

// 定义插件相关的类型接口（这些类型在原组件中似乎没有明确定义，需要根据实际使用推断）
interface TeaAssistantTypePlugin {
    onAssistantTypeRun?: (assistantRunApi: any) => void;
}

interface TeaPlugin {
    // 基础插件接口
}

interface FieldConfig {
    // 字段配置接口
}

interface AssistantTypeApi {
    typeRegist: (
        code: number,
        name: string,
        pluginInstance: TeaAssistantTypePlugin & TeaPlugin,
    ) => void;
    markdownRemarkRegist: (remark: any) => void;
    changeFieldLabel: (fieldName: string, label: string) => void;
    addField: (
        fieldName: string,
        label: string,
        defaultValue: string,
        config?: FieldConfig,
    ) => void;
    addFieldTips: (fieldName: string, tips: string) => void;
    hideField: (fieldName: string) => void;
    runLogic: (logic: (assistantRunApi: any) => void) => void;
    forceFieldValue: (fieldName: string, value: string) => void;
}

interface AssistantRunApi {
    // 助手运行时API的接口定义
    [key: string]: any;
}

// 用于存储AskAssistantApi中对应的处理函数
interface AskAssistantApiFunctions {
    onCustomUserMessage?: (
        question: string,
        assistantId: string,
        conversationId?: string,
    ) => any;
    onCustomUserMessageComing?: (aiResponse: any) => void;
    onStreamMessageListener?: (
        payload: string,
        aiResponse: any,
        responseIsResponsingFunction: (isFinish: boolean) => void,
    ) => void;
}

export interface UsePluginManagementReturn {
    assistantTypePluginMap: Map<number, TeaAssistantTypePlugin>;
    functionMap: Map<number, AskAssistantApiFunctions>;
    assistantTypeApi: AssistantTypeApi;
    setFunctionMapForMessage: (messageId: number) => void;
    getAssistantPlugin: (assistantType: number) => TeaAssistantTypePlugin | undefined;
}

export function usePluginManagement(pluginList: any[]): UsePluginManagementReturn {
    // 助手类型插件映射表，key为助手类型，value为插件实例
    const [assistantTypePluginMap, setAssistantTypePluginMap] = useState<
        Map<number, TeaAssistantTypePlugin>
    >(new Map());

    // 插件函数映射表，用于存储每个消息对应的处理函数
    const [functionMap, setFunctionMap] = useState<
        Map<number, AskAssistantApiFunctions>
    >(new Map());

    // 助手类型API接口，提供给插件使用
    const assistantTypeApi: AssistantTypeApi = {
        typeRegist: (
            code: number,
            _: string,
            pluginInstance: TeaAssistantTypePlugin & TeaPlugin,
        ) => {
            setAssistantTypePluginMap((prev) => {
                const newMap = new Map(prev);
                newMap.set(code, pluginInstance);
                return newMap;
            });
        },
        markdownRemarkRegist: (_: any) => { },
        changeFieldLabel: (_: string, __: string) => { },
        addField: (
            _: string,
            __: string,
            ___: string,
            ____?: FieldConfig,
        ) => { },
        addFieldTips: (_: string, __: string) => { },
        hideField: (_: string) => { },
        runLogic: (_: (assistantRunApi: AssistantRunApi) => void) => { },
        forceFieldValue: (_: string, __: string) => { },
    };

    // 为指定消息设置函数映射
    const setFunctionMapForMessage = useCallback((messageId: number) => {
        setFunctionMap((prev) => {
            const newMap = new Map(prev);
            newMap.set(messageId, {
                onCustomUserMessage: undefined,
                onCustomUserMessageComing: undefined,
                onStreamMessageListener: undefined,
            });
            return newMap;
        });
    }, []);

    // 获取指定助手类型的插件实例
    const getAssistantPlugin = useCallback((assistantType: number) => {
        return assistantTypePluginMap.get(assistantType);
    }, [assistantTypePluginMap]);

    // 初始化助手类型插件
    useEffect(() => {
        pluginList
            .filter((plugin: any) =>
                plugin.pluginType.includes("assistantType"),
            )
            .forEach((plugin: any) => {
                plugin.instance?.onAssistantTypeInit(assistantTypeApi);
            });
    }, [pluginList]);

    return {
        assistantTypePluginMap,
        functionMap,
        assistantTypeApi,
        setFunctionMapForMessage,
        getAssistantPlugin,
    };
}