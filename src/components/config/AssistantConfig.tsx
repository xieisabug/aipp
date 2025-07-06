import React, { useCallback, useEffect, useState, useMemo } from "react";
import { toast } from "sonner";
import { invoke } from "@tauri-apps/api/core";
import { AssistantDetail, AssistantListItem } from "../../data/Assistant";
import ConfigForm from "../ConfigForm";
import ConfirmDialog from "../ConfirmDialog";
import AddAssistantDialog from "./AddAssistantDialog";
import EditAssistantDialog from "./EditAssistantDialog";
import { AssistantType } from "../../types/assistant";
import { validateConfig } from "../../utils/validate";
import { Bot, Settings, User } from "lucide-react";

// 导入公共组件
import {
    ConfigPageLayout,
    SidebarList,
    ListItemButton,
    EmptyState,
    SelectOption
} from "../common";

import "../../styles/AssistantConfig.css";
import { useForm } from "react-hook-form";

interface ModelForSelect {
    name: string;
    code: string;
    id: number;
    llm_provider_id: number;
}

interface AssistantConfigProps {
    pluginList: any[];
}
const AssistantConfig: React.FC<AssistantConfigProps> = ({ pluginList }) => {
    // 插件加载部分
    // 插件实例
    const [assistantTypePluginMap, setAssistantTypePluginMap] = useState<
        Map<number, TeaAssistantTypePlugin>
    >(new Map());
    // 插件名称
    const [assistantTypeNameMap, setAssistantTypeNameMap] = useState<
        Map<number, string>
    >(new Map<number, string>());
    // 插件自定义字段
    const [assistantTypeCustomField, setAssistantTypeCustomField] = useState<
        Array<{ key: string; value: Record<string, any> }>
    >([]);
    // 插件自定义label
    const [assistantTypeCustomLabel, setAssistantTypeCustomLabel] = useState<
        Map<string, string>
    >(new Map<string, string>());
    // 插件自定义tips
    const [assistantTypeCustomTips, setAssistantTypeCustomTips] = useState<
        Map<string, string>
    >(new Map<string, string>());
    // 插件隐藏字段
    const [assistantTypeHideField, setAssistantTypeHideField] = useState<
        Array<string>
    >([]);
    const form = useForm();

    // 使用 useMemo 缓存 assistantTypeApi
    const assistantTypeApi: AssistantTypeApi = useMemo(
        () => ({
            typeRegist: (
                code: number,
                label: string,
                pluginInstance: TeaAssistantTypePlugin & TeaPlugin,
            ) => {
                // 检查是否已存在相同的 code
                setAssistantTypes((prev) => {
                    if (!prev.some((type) => type.code === code)) {
                        return [...prev, { code: code, name: label }];
                    } else {
                        return prev;
                    }
                });

                setAssistantTypePluginMap((prev) => {
                    const newMap = new Map(prev);
                    newMap.set(code, pluginInstance);
                    return newMap;
                });
                setAssistantTypeNameMap((prev) => {
                    const newMap = new Map(prev);
                    newMap.set(code, label);
                    return newMap;
                });
            },
            markdownRemarkRegist: (_: any) => { },
            changeFieldLabel: (fieldName: string, label: string) => {
                setAssistantTypeCustomLabel((prev) => {
                    const newMap = new Map(prev);
                    newMap.set(fieldName, label);
                    return newMap;
                });
            },
            addField: (
                fieldName: string,
                label: string,
                type: string,
                fieldConfig?: FieldConfig,
            ) => {
                setAssistantTypeCustomField((prev) => {
                    const newField = {
                        key: fieldName,
                        value: Object.assign(
                            {
                                type: type,
                                label: label,
                                value: "",
                            },
                            fieldConfig,
                        ),
                    };
                    return [...prev, newField];
                });
            },
            hideField: (fieldName: string) => {
                setAssistantTypeHideField((prev) => {
                    return [...prev, fieldName];
                });
            },
            addFieldTips: (fieldName: string, tips: string) => {
                setAssistantTypeCustomTips((prev) => {
                    const newMap = new Map(prev);
                    newMap.set(fieldName, tips);
                    return newMap;
                });
            },
            runLogic: (_: (assistantRunApi: AssistantRunApi) => void) => { },
            forceFieldValue: function (_: string, __: string): void { },
        }),
        [],
    );
    // 给默认的字段增加Label和Tips
    useEffect(() => {
        assistantTypeApi.changeFieldLabel("max_tokens", "Max Tokens");
        assistantTypeApi.changeFieldLabel("temperature", "Temperature");
        assistantTypeApi.changeFieldLabel("top_p", "Top P");
        assistantTypeApi.changeFieldLabel("stream", "Stream");
        assistantTypeApi.addFieldTips("max_tokens", "最大Token数，影响回复的长度");
        assistantTypeApi.addFieldTips("temperature", "控制生成的随机性，越高越随机");
        assistantTypeApi.addFieldTips("top_p", "控制生成的多样性，越高越多样");
        assistantTypeApi.addFieldTips("stream", "是否流式输出，开启后可能会有延迟");
    }, [assistantTypeApi]);

    // 助手类型
    const [assistantTypes, setAssistantTypes] = useState<AssistantType[]>([
        { code: 0, name: "普通对话助手" },
    ]);
    // 加载助手类型的插件
    useEffect(() => {
        pluginList
            .filter((plugin: any) =>
                plugin.pluginType.includes("assistantType"),
            )
            .forEach((plugin: any) => {
                plugin?.instance?.onAssistantTypeInit(assistantTypeApi);
            });
    }, [pluginList]);

    // 模型数据
    const [models, setModels] = useState<ModelForSelect[]>([]);
    useEffect(() => {
        // 获取模型列表
        invoke<Array<ModelForSelect>>("get_models_for_select")
            .then(setModels)
            .catch((error) => {
                toast.error("获取模型列表失败: " + error);
            });
    }, []);

    // 当前助手
    const [currentAssistant, setCurrentAssistant] =
        useState<AssistantDetail | null>(null);

    const assistantConfigApi: AssistantConfigApi = useMemo(
        () => ({
            clearFieldValue: function (fieldName: string): void {
                handleConfigChange(fieldName, "", "");
            },
            changeFieldValue: function (
                fieldName: string,
                value: any,
                valueType: string,
            ): void {
                console.log("changeFieldValue", fieldName, value, valueType);
                handleConfigChange(fieldName, value, valueType);
            },
        }),
        [form, currentAssistant],
    );

    // 助手相关
    // 助手列表
    const [assistants, setAssistants] = useState<AssistantListItem[]>([]);
    useEffect(() => {
        invoke<Array<AssistantListItem>>("get_assistants")
            .then((assistantList) => {
                setAssistants(assistantList);

                if (assistantList.length) {
                    handleChooseAssistant(assistantList[0]);
                }
            })
            .catch((error) => {
                toast.error("获取助手列表失败: " + error);
            });
    }, []);
    // 使用 useCallback 缓存回调函数
    const onSave = useCallback((assistant: AssistantDetail) => {
        return invoke<void>("save_assistant", { assistantDetail: assistant });
    }, []);
    // 复制助手
    const onCopy = useCallback(() => {
        invoke<AssistantDetail>("copy_assistant", {
            assistantId: currentAssistant?.assistant.id,
        })
            .then((assistantDetail: AssistantDetail) => {
                setAssistants((prev) => [
                    ...prev,
                    {
                        id: assistantDetail.assistant.id,
                        name: assistantDetail.assistant.name,
                    },
                ]);
                setCurrentAssistant(assistantDetail);
                toast.success("复制助手成功");
            })
            .catch((error) => {
                toast.error("复制助手失败: " + error);
            });
    }, [currentAssistant]);

    // 点击某个助手后，选择助手
    const handleChooseAssistant = useCallback(
        (assistant: AssistantListItem) => {
            if (
                !currentAssistant ||
                currentAssistant.assistant.id !== assistant.id
            ) {
                invoke<AssistantDetail>("get_assistant", {
                    assistantId: assistant.id,
                })
                    .then((assistant: AssistantDetail) => {
                        setCurrentAssistant(assistant);
                        form.reset({
                            assistantType: assistant.assistant.assistant_type,
                            model:
                                assistant.model.length > 0
                                    ? `${assistant.model[0].model_code}%%${assistant.model[0].provider_id}`
                                    : "-1",
                            prompt: assistant.prompts[0].prompt,
                            ...assistant.model_configs.reduce(
                                (acc, config) => {
                                    acc[config.name] =
                                        config.value_type === "boolean"
                                            ? config.value == "true"
                                            : config.value;
                                    return acc;
                                },
                                {} as Record<string, any>,
                            ),
                            ...assistantTypeCustomField.reduce(
                                (acc, field) => {
                                    acc[field.key] =
                                        field.value.type === "checkbox"
                                            ? assistant.model_configs.find(
                                                (config) =>
                                                    config.name === field.key,
                                            )?.value === "true"
                                            : (assistant.model_configs.find(
                                                (config) =>
                                                    config.name === field.key,
                                            )?.value ?? "");
                                    return acc;
                                },
                                {} as Record<string, any>,
                            ),
                        });
                        setAssistantTypeCustomField([]);
                        assistantTypePluginMap
                            .get(assistant.assistant.assistant_type)
                            ?.onAssistantTypeSelect(assistantTypeApi);
                    })
                    .catch((error) => {
                        toast.error("获取助手信息失败: " + error);
                    });
            }
        },
        [
            currentAssistant,
            assistantTypeCustomField,
            assistantTypePluginMap,
            assistantTypeApi,
            form,
        ],
    );

    // 修改配置
    const handleConfigChange = useCallback(
        (key: string, value: string | boolean, value_type: string) => {
            console.log(
                "handleConfigChange",
                key,
                value,
                value_type,
                currentAssistant,
            );
            if (currentAssistant) {
                const index = currentAssistant.model_configs.findIndex(
                    (config) => config.name === key,
                );
                const { isValid, parsedValue } = validateConfig(
                    value,
                    value_type,
                );
                if (!isValid) return;

                // 更新表单值
                form.setValue(key, parsedValue);
                // 更新模型配置
                setCurrentAssistant((prev) => {
                    if (!prev) return prev;
                    const newConfigs =
                        index !== -1
                            ? prev.model_configs.map((config, i) =>
                                i === index
                                    ? {
                                        ...config,
                                        value: parsedValue.toString(),
                                    }
                                    : config,
                            )
                            : [
                                ...prev.model_configs,
                                {
                                    name: key,
                                    value: parsedValue.toString(),
                                    value_type: value_type,
                                    id: 0,
                                    assistant_id: prev.assistant.id,
                                    assistant_model_id:
                                        prev.model[0]?.id ?? 0,
                                },
                            ];
                    return { ...prev, model_configs: newConfigs };
                });
            }
        },
        [currentAssistant],
    );

    // 修改 prompt
    const handlePromptChange = useCallback(
        (value: string) => {
            if (!currentAssistant?.prompts.length) return;

            setCurrentAssistant((prev) => {
                if (!prev) return prev;
                return {
                    ...prev,
                    prompts: [
                        {
                            ...prev.prompts[0],
                            prompt: value,
                        },
                    ],
                };
            });
        },
        [currentAssistant],
    );

    // 保存助手
    const handleAssistantFormSave = useCallback(() => {
        if (!currentAssistant) return;

        const values = form.getValues();

        onSave({
            ...currentAssistant,
            assistant: {
                ...currentAssistant.assistant,
                assistant_type: values.assistantType,
                name: currentAssistant.assistant.name,
                description: currentAssistant.assistant.description,
            },
            model: [
                {
                    ...currentAssistant.model[0],
                    model_code: values.model.split("%%")[0],
                    provider_id: parseInt(values.model.split("%%")[1]),
                    alias: "",
                },
            ],
            model_configs: Object.entries(values)
                .filter(
                    ([key]) =>
                        key !== "assistantType" &&
                        key !== "model" &&
                        key !== "prompt",
                )
                .filter(([key]) => {
                    // 确保 key 是可存储的值，不是static和button
                    const config = currentAssistant.model_configs.find(
                        (config) => config.name === key,
                    );
                    return (
                        config && config.value_type &&
                        config?.value_type !== "static" &&
                        config?.value_type !== "button" &&
                        config?.value_type !== "custom"
                    );
                })
                .map(([key, value]) => {
                    const config = currentAssistant.model_configs.find(
                        (config) => config.name === key,
                    );
                    return {
                        name: key,
                        value: value.toString(),
                        value_type: config?.value_type ?? "string",
                        id: config?.id ?? 0,
                        assistant_id: currentAssistant.assistant.id,
                        assistant_model_id: currentAssistant.model[0]?.id ?? 0,
                    };
                }),
            prompts: [
                {
                    ...currentAssistant.prompts[0],
                    prompt: values.prompt,
                },
            ],
        })
            .then(() => toast.success("保存成功"))
            .catch((error) => toast.error("保存失败: " + error));
    }, [currentAssistant, form, onSave]);

    // 删除助手
    const [confirmDeleteDialogIsOpen, setConfirmDeleteDialogIsOpen] =
        useState<boolean>(false);
    const closeConfirmDeleteDialog = useCallback(() => {
        setConfirmDeleteDialogIsOpen(false);
    }, []);
    // 打开删除助手对话框
    const openConfirmDeleteDialog = useCallback(() => {
        setConfirmDeleteDialogIsOpen(true);
    }, []);
    const handleDelete = useCallback(() => {
        if (currentAssistant) {
            invoke("delete_assistant", {
                assistantId: currentAssistant.assistant.id,
            })
                .then(() => {
                    const newAssistants = assistants.filter(
                        (assistant) =>
                            assistant.id !== currentAssistant.assistant.id,
                    );
                    setAssistants(newAssistants);
                    if (newAssistants.length > 0) {
                        handleChooseAssistant(newAssistants[0]);
                    } else {
                        setCurrentAssistant(null);
                    }
                    setConfirmDeleteDialogIsOpen(false);
                    toast.success("删除助手成功");
                })
                .catch((error) => {
                    toast.error("删除助手失败: " + error);
                });
        }
    }, [currentAssistant, assistants]);

    // 修改助手名称与描述
    const [updateFormDialogIsOpen, setUpdateFormDialogIsOpen] =
        useState<boolean>(false);
    const openUpdateFormDialog = useCallback(() => {
        setUpdateFormDialogIsOpen(true);
    }, []);
    const closeUpdateFormDialog = useCallback(() => {
        setUpdateFormDialogIsOpen(false);
    }, []);

    const handleAssistantUpdated = useCallback(
        (updatedAssistant: AssistantDetail) => {
            setCurrentAssistant(updatedAssistant);
            const index = assistants.findIndex(
                (assistant) => assistant.id === updatedAssistant.assistant.id,
            );
            if (index >= 0) {
                const newAssistants = [...assistants];
                newAssistants[index] = {
                    id: updatedAssistant.assistant.id,
                    name: updatedAssistant.assistant.name,
                };
                setAssistants(newAssistants);
            }
        },
        [assistants],
    );

    // 助手配置表单
    const modelOptions = useMemo(
        () =>
            models.map((m) => ({
                value: `${m.code}%%${m.llm_provider_id}`,
                label: m.name,
            })),
        [models],
    );
    // 使用 useMemo 缓存表单配置
    const assistantFormConfig = useMemo(() => {
        if (!currentAssistant) return [];

        let configs = [
            {
                key: "assistantType",
                config: {
                    type: "static" as const,
                    label:
                        assistantTypeCustomLabel.get("assistantType") ??
                        "助手类型",
                    value:
                        assistantTypeNameMap.get(
                            currentAssistant?.assistant.assistant_type ?? 0,
                        ) ?? "普通对话助手",
                },
            },
            {
                key: "model",
                config: {
                    type: "select" as const,
                    label: assistantTypeCustomLabel.get("model") ?? "Model",
                    options: modelOptions,
                    value:
                        (currentAssistant?.model.length ?? 0 > 0)
                            ? `${currentAssistant?.model[0].model_code}%%${currentAssistant?.model[0].provider_id}`
                            : "-1",
                    onChange: (value: string | boolean) => {
                        const [modelCode, providerId] = (value as string).split(
                            "%%",
                        );
                        if (currentAssistant?.model.length ?? 0 > 0) {
                            let assistant = currentAssistant as AssistantDetail;
                            setCurrentAssistant({
                                ...assistant,
                                model: [
                                    {
                                        ...assistant?.model[0],
                                        model_code: modelCode,
                                        provider_id: parseInt(providerId),
                                    },
                                ],
                            });
                        } else {
                            let assistant = currentAssistant as AssistantDetail;
                            setCurrentAssistant({
                                ...assistant,
                                model: [
                                    {
                                        id: 0,
                                        assistant_id: assistant.assistant.id,
                                        model_code: modelCode,
                                        provider_id: parseInt(providerId),
                                        alias: "",
                                    },
                                ],
                            });
                        }
                    },
                },
            },
            ...currentAssistant?.model_configs
                .filter(
                    (config) => !assistantTypeHideField.includes(config.name) && !assistantTypeCustomField.find((field) => field.key === config.name),
                )
                .map((config) => ({
                    key: config.name,
                    config: {
                        type:
                            config.value_type === "boolean"
                                ? ("checkbox" as const)
                                : ("input" as const),
                        label:
                            assistantTypeCustomLabel.get(config.name) ??
                            config.name,
                        value:
                            config.value_type === "boolean"
                                ? config.value == "true"
                                : config.value,
                        tooltip: assistantTypeCustomTips.get(config.name),
                        onChange: (value: string | boolean) =>
                            handleConfigChange(
                                config.name,
                                value,
                                config.value_type,
                            ),
                        onBlur: (value: string | boolean) =>
                            handleConfigChange(
                                config.name,
                                value as string,
                                config.value_type,
                            ),
                    },
                })),
            ...assistantTypeCustomField
                .filter((field) => !assistantTypeHideField.includes(field.key))
                .map((field) => ({
                    key: field.key,
                    config: {
                        ...field.value,
                        type: field.value.type,
                        label:
                            assistantTypeCustomLabel.get(field.key) ??
                            field.value.label,
                        value: (() => {
                            const config = currentAssistant?.model_configs.find(
                                (config) => config.name === field.key,
                            );
                            if (field.value.type === "checkbox") {
                                return config?.value === "true";
                            } else if (field.value.type === "static") {
                                return config?.value;
                            } else {
                                return config?.value ?? field.value.value ?? "";
                            }
                        })(),
                        tooltip: assistantTypeCustomTips.get(field.key),
                        onChange: (value: string | boolean) =>
                            handleConfigChange(
                                field.key,
                                value,
                                field.value.type === "checkbox"
                                    ? "boolean"
                                    : "string",
                            ),
                        onBlur: (value: string | boolean) =>
                            handleConfigChange(
                                field.key,
                                value as string,
                                field.value.type === "checkbox"
                                    ? "boolean"
                                    : "string",
                            ),
                    },
                })),
            {
                key: "prompt",
                config: {
                    type: "textarea" as const,
                    label: assistantTypeCustomLabel.get("prompt") ?? "Prompt",
                    className: "h-64",
                    value: currentAssistant?.prompts[0].prompt ?? "",
                    onChange: (value: string | boolean) =>
                        handlePromptChange(value as string),
                },
            },
        ];

        return configs;
    }, [
        currentAssistant,
        assistantTypeNameMap,
        assistantTypeCustomField,
        assistantTypeCustomLabel,
        assistantTypeHideField,
        modelOptions,
        handleConfigChange,
        handlePromptChange,
    ]);

    // 添加新的处理函数
    const handleAssistantAdded = (assistantDetail: AssistantDetail) => {
        setAssistants((prev) => [
            ...prev,
            {
                id: assistantDetail.assistant.id,
                name: assistantDetail.assistant.name,
            },
        ]);
        setCurrentAssistant(assistantDetail);
    };

    // 下拉菜单选项
    const selectOptions: SelectOption[] = useMemo(() =>
        assistants.map(assistant => ({
            id: assistant.id.toString(),
            label: assistant.name,
            icon: <User className="h-4 w-4" />
        })), [assistants]);

    // 下拉菜单选择回调
    const handleSelectFromDropdown = useCallback((assistantId: string) => {
        const assistant = assistants.find(a => a.id.toString() === assistantId);
        if (assistant) {
            handleChooseAssistant(assistant);
        }
    }, [assistants, handleChooseAssistant]);

    // 新增按钮组件
    const addButton = useMemo(() => (
        <AddAssistantDialog
            assistantTypes={assistantTypes}
            onAssistantAdded={handleAssistantAdded}
            triggerButtonProps={{
                className: "gap-2 bg-gray-800 hover:bg-gray-900 text-white shadow-sm hover:shadow-md transition-all"
            }}
        />
    ), [assistantTypes, handleAssistantAdded]);

    // 空状态
    if (assistants.length === 0) {
        return (
            <ConfigPageLayout
                sidebar={null}
                content={
                    <EmptyState
                        icon={<Bot className="h-8 w-8 text-gray-500" />}
                        title="还没有配置助手"
                        description="创建你的第一个AI助手，开始享受个性化的智能对话体验"
                        action={
                            <AddAssistantDialog
                                assistantTypes={assistantTypes}
                                onAssistantAdded={handleAssistantAdded}
                            />
                        }
                    />
                }
            />
        );
    }

    // 侧边栏内容
    const sidebar = (
        <SidebarList
            title="助手列表"
            description="选择助手进行配置"
            icon={<Bot className="h-5 w-5" />}
        >
            {assistants.map((assistant, index) => (
                <ListItemButton
                    key={index}
                    isSelected={currentAssistant?.assistant.id === assistant.id}
                    onClick={() => handleChooseAssistant(assistant)}
                >
                    <User className="h-4 w-4 mr-2 flex-shrink-0" />
                    <span className="truncate">{assistant.name}</span>
                </ListItemButton>
            ))}
        </SidebarList>
    );

    // 右侧内容
    const content = currentAssistant ? (
        <ConfigForm
            assistantConfigApi={assistantConfigApi}
            title={currentAssistant.assistant.name}
            description={currentAssistant.assistant.description || "配置你的智能助手"}
            config={assistantFormConfig}
            layout="prompt"
            classNames="bottom-space"
            onSave={handleAssistantFormSave}
            onCopy={currentAssistant.assistant.id === 1 ? undefined : onCopy}
            onDelete={currentAssistant.assistant.id === 1 ? undefined : openConfirmDeleteDialog}
            onEdit={openUpdateFormDialog}
            useFormReturn={form}
        />
    ) : (
        <EmptyState
            icon={<Settings className="h-8 w-8 text-gray-500" />}
            title="选择一个助手"
            description="从左侧列表中选择一个助手开始配置"
        />
    );

    return (
        <>
            <ConfigPageLayout
                sidebar={sidebar}
                content={content}
                selectOptions={selectOptions}
                selectedOptionId={currentAssistant?.assistant.id.toString()}
                onSelectOption={handleSelectFromDropdown}
                selectPlaceholder="选择助手"
                addButton={addButton}
            />

            {/* 对话框 */}
            <ConfirmDialog
                title="确认删除"
                confirmText="该操作不可逆，确认执行删除助手操作吗？删除后，配置将会删除，并且该助手的对话将转移到快速使用助手，且不可恢复。"
                onConfirm={handleDelete}
                onCancel={closeConfirmDeleteDialog}
                isOpen={confirmDeleteDialogIsOpen}
            />

            <EditAssistantDialog
                isOpen={updateFormDialogIsOpen}
                onClose={closeUpdateFormDialog}
                currentAssistant={currentAssistant}
                onSave={onSave}
                onAssistantUpdated={handleAssistantUpdated}
            />
        </>
    );
};

export default AssistantConfig;
