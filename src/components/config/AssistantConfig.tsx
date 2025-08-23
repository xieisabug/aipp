import React, { useCallback, useEffect, useMemo } from "react";
import { toast } from "sonner";
import { AssistantDetail, AssistantListItem } from "../../data/Assistant";
import { useAssistantListListener } from "../../hooks/useAssistantListListener";
import { Bot, Settings, User, Download } from "lucide-react";
import { Button } from "../ui/button";
import { useForm } from "react-hook-form";
import { validateConfig } from "../../utils/validate";
import AddAssistantDialog from "./AddAssistantDialog";

// 导入公共组件
import { ConfigPageLayout, SidebarList, ListItemButton, EmptyState, SelectOption } from "../common";

// 导入新的 hooks 和组件
import { useAssistantTypePlugin } from "@/hooks/assistant/useAssistantTypePlugin";
import { useAssistantOperations } from "@/hooks/assistant/useAssistantOperations";
import { useAssistantFormConfig } from "@/hooks/assistant/useAssistantFormConfig";
import { useDialogStates } from "@/hooks/assistant/useDialogStates";
import { AssistantFormRenderer } from "./assistant/AssistantFormRenderer";
import { AssistantDialogs } from "./assistant/AssistantDialogs";
import { AssistantConfigApi } from "@/types/forms";

interface AssistantConfigProps {
    pluginList: any[];
    navigateTo: (menuKey: string) => void;
}
const AssistantConfig: React.FC<AssistantConfigProps> = ({ pluginList, navigateTo }) => {
    const form = useForm();

    const {
        assistantTypes,
        assistantTypePluginMap,
        assistantTypeNameMap,
        assistantTypeCustomField,
        setAssistantTypeCustomField,
        assistantTypeCustomLabel,
        assistantTypeCustomTips,
        assistantTypeHideField,
        assistantTypeApi,
    } = useAssistantTypePlugin(pluginList);

    const {
        assistants,
        currentAssistant,
        setCurrentAssistant,
        saveAssistant,
        copyAssistant,
        deleteAssistant,
        loadAssistants,
        loadAssistantDetail,
        shareAssistant,
        importAssistant,
        updateAssistantInfo,
        addAssistant,
    } = useAssistantOperations();

    const {
        dialogStates,
        shareCode,
        openConfirmDeleteDialog,
        closeConfirmDeleteDialog,
        openUpdateFormDialog,
        closeUpdateFormDialog,
        openShareDialog,
        closeShareDialog,
        openImportDialog,
        closeImportDialog,
    } = useDialogStates();

    // 助手配置 API
    const assistantConfigApi: AssistantConfigApi = useMemo(
        () => ({
            clearFieldValue: function (fieldName: string): void {
                handleConfigChange(fieldName, "", "");
            },
            changeFieldValue: function (fieldName: string, value: any, valueType: string): void {
                console.log("changeFieldValue", fieldName, value, valueType);
                handleConfigChange(fieldName, value, valueType);
            },
        }),
        [currentAssistant]
    );

    // 初始化助手列表
    useEffect(() => {
        loadAssistants().then((assistantList) => {
            if (assistantList.length) {
                handleChooseAssistant(assistantList[0]);
            }
        });
    }, [loadAssistants]);
    // 监听助手列表变化
    useAssistantListListener({
        onAssistantListChanged: useCallback(
            (assistantList: AssistantListItem[]) => {
                // 使用 operations hook 的方法更新列表
                // 这里需要手动更新，因为 hook 内部不知道这个事件
                if (assistantList.length > 0) {
                    const currentAssistantExists = assistantList.some(
                        (assistant) => assistant.id === currentAssistant?.assistant.id
                    );
                    if (!currentAssistantExists) {
                        handleChooseAssistant(assistantList[0]);
                    }
                } else {
                    setCurrentAssistant(null);
                }
            },
            [currentAssistant?.assistant.id]
        ),
    });

    // 选择助手
    const handleChooseAssistant = useCallback(
        (assistant: AssistantListItem) => {
            if (!currentAssistant || currentAssistant.assistant.id !== assistant.id) {
                loadAssistantDetail(assistant.id).then((assistantDetail) => {
                    form.reset({
                        assistantType: assistantDetail.assistant.assistant_type,
                        model:
                            assistantDetail.model.length > 0
                                ? `${assistantDetail.model[0].model_code}%%${assistantDetail.model[0].provider_id}`
                                : "-1",
                        prompt: assistantDetail.prompts[0].prompt,
                        ...assistantDetail.model_configs.reduce((acc, config) => {
                            acc[config.name] = config.value_type === "boolean" ? config.value == "true" : config.value;
                            return acc;
                        }, {} as Record<string, any>),
                        ...assistantTypeCustomField.reduce((acc, field) => {
                            acc[field.key] =
                                field.value.type === "checkbox"
                                    ? assistantDetail.model_configs.find((config) => config.name === field.key)
                                          ?.value === "true"
                                    : assistantDetail.model_configs.find((config) => config.name === field.key)
                                          ?.value ?? "";
                            return acc;
                        }, {} as Record<string, any>),
                    });
                    setAssistantTypeCustomField([]);
                    const plugin = assistantTypePluginMap.get(assistantDetail.assistant.assistant_type);
                    plugin?.onAssistantTypeSelect?.(assistantTypeApi);
                });
            }
        },
        [
            currentAssistant,
            assistantTypeCustomField,
            assistantTypePluginMap,
            assistantTypeApi,
            form,
            loadAssistantDetail,
        ]
    );

    // 修改配置
    const handleConfigChange = useCallback(
        (key: string, value: string | boolean, value_type: string) => {
            console.log("handleConfigChange", key, value, value_type, currentAssistant);
            if (currentAssistant) {
                const index = currentAssistant.model_configs.findIndex((config) => config.name === key);
                const { isValid, parsedValue } = validateConfig(value, value_type);
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
                                      : config
                              )
                            : [
                                  ...prev.model_configs,
                                  {
                                      name: key,
                                      value: parsedValue.toString(),
                                      value_type: value_type,
                                      id: 0,
                                      assistant_id: prev.assistant.id,
                                      assistant_model_id: prev.model[0]?.id ?? 0,
                                  },
                              ];
                    return { ...prev, model_configs: newConfigs };
                });
            }
        },
        [currentAssistant, form, setCurrentAssistant]
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
        [currentAssistant, setCurrentAssistant]
    );

    // 使用新的 hook 生成表单配置
    const { formConfig } = useAssistantFormConfig({
        currentAssistant,
        assistantTypeNameMap,
        assistantTypeCustomField,
        assistantTypeCustomLabel,
        assistantTypeCustomTips,
        assistantTypeHideField,
        navigateTo,
        onConfigChange: handleConfigChange,
        onPromptChange: handlePromptChange,
    });

    // 保存助手
    const handleAssistantFormSave = useCallback(() => {
        if (!currentAssistant) return;

        const values = form.getValues();

        saveAssistant({
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
                    ([key]) => key !== "assistantType" && key !== "model" && key !== "prompt" && key !== "mcp_config"
                )
                .filter(([key]) => {
                    const config = currentAssistant.model_configs.find((config) => config.name === key);
                    const customField = assistantTypeCustomField.find((field) => field.key === key);
                    
                    // 如果是插件自定义字段，直接允许保存
                    if (customField) {
                        return true;
                    }
                    
                    // 原有的过滤逻辑
                    return (
                        config &&
                        config.value_type &&
                        config?.value_type !== "static" &&
                        config?.value_type !== "button" &&
                        config?.value_type !== "custom"
                    );
                })
                .map(([key, value]) => {
                    const config = currentAssistant.model_configs.find((config) => config.name === key);
                    const customField = assistantTypeCustomField.find((field) => field.key === key);
                    
                    // 为插件自定义字段确定正确的 value_type
                    let valueType = config?.value_type ?? "string";
                    if (customField) {
                        // 根据插件字段的类型映射到数据库的 value_type
                        const fieldType = customField.value.type;
                        if (fieldType === "checkbox" || fieldType === "switch") {
                            valueType = "boolean";
                        } else if (fieldType === "select" || fieldType === "radio") {
                            valueType = "string";
                        } else {
                            valueType = "string";
                        }
                    }
                    
                    return {
                        name: key,
                        value: value ? value.toString() : null,
                        value_type: valueType,
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
    }, [currentAssistant, form, saveAssistant, assistantTypeCustomField]);

    // 删除助手
    const handleDelete = useCallback(() => {
        deleteAssistant()
            .then((result) => {
                if (result.shouldSelectFirst && result.assistants.length > 0) {
                    handleChooseAssistant(result.assistants[0]);
                }
                closeConfirmDeleteDialog();
            })
            .catch(() => {
                // 错误已在 hook 中处理
            });
    }, [deleteAssistant, closeConfirmDeleteDialog, handleChooseAssistant]);

    // 添加新助手处理
    const handleAssistantAdded = useCallback(
        (assistantDetail: AssistantDetail) => {
            addAssistant(assistantDetail);

            // 重置表单状态为新助手的配置
            form.reset({
                assistantType: assistantDetail.assistant.assistant_type,
                model:
                    assistantDetail.model.length > 0
                        ? `${assistantDetail.model[0].model_code}%%${assistantDetail.model[0].provider_id}`
                        : "-1",
                prompt: assistantDetail.prompts[0]?.prompt || "",
                ...assistantDetail.model_configs.reduce((acc, config) => {
                    acc[config.name] = config.value_type === "boolean" ? config.value == "true" : config.value;
                    return acc;
                }, {} as Record<string, any>),
            });
        },
        [addAssistant, form]
    );

    // 侧边栏内容
    const sidebar = (
        <SidebarList title="助手列表" description="选择助手进行配置" icon={<Bot className="h-5 w-5" />}>
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

    // 分享助手
    const handleShareAssistant = useCallback(async () => {
        try {
            const code = await shareAssistant();
            openShareDialog(code);
        } catch (error) {
            // 错误已在 hook 中处理
        }
    }, [shareAssistant, openShareDialog]);

    // 下拉菜单选项
    const selectOptions: SelectOption[] = useMemo(
        () =>
            assistants.map((assistant) => ({
                id: assistant.id.toString(),
                label: assistant.name,
                icon: <User className="h-4 w-4" />,
            })),
        [assistants]
    );

    // 下拉菜单选择回调
    const handleSelectFromDropdown = useCallback(
        (assistantId: string) => {
            const assistant = assistants.find((a) => a.id.toString() === assistantId);
            if (assistant) {
                handleChooseAssistant(assistant);
            }
        },
        [assistants, handleChooseAssistant]
    );

    // 新增按钮组件
    const addButton = useMemo(
        () => (
            <div className="flex gap-2">
                <AddAssistantDialog
                    assistantTypes={assistantTypes}
                    onAssistantAdded={handleAssistantAdded}
                    triggerButtonProps={{
                        className:
                            "gap-2 bg-primary hover:bg-primary/90 text-primary-foreground shadow-sm hover:shadow-md transition-all",
                    }}
                />
                <Button
                    variant="outline"
                    onClick={openImportDialog}
                    className="shadow-sm hover:shadow-md transition-all"
                >
                    <Download className="h-4 w-4" />
                </Button>
            </div>
        ),
        [assistantTypes, handleAssistantAdded, openImportDialog]
    );

    // 空状态
    if (assistants.length === 0) {
        return (
            <>
                <ConfigPageLayout
                    sidebar={null}
                    content={
                        <EmptyState
                            icon={<Bot className="h-8 w-8 text-muted-foreground" />}
                            title="还没有配置助手"
                            description="创建你的第一个AI助手，开始享受个性化的智能对话体验"
                            action={
                                <div className="flex flex-col gap-3">
                                    <div className="flex gap-2 justify-center">
                                        <AddAssistantDialog
                                            assistantTypes={assistantTypes}
                                            onAssistantAdded={handleAssistantAdded}
                                        />
                                        <Button
                                            variant="outline"
                                            onClick={openImportDialog}
                                            className="shadow-lg hover:shadow-xl transition-all"
                                        >
                                            <Download className="h-4 w-4" />
                                        </Button>
                                    </div>
                                </div>
                            }
                        />
                    }
                />

                <AssistantDialogs
                    dialogStates={dialogStates}
                    shareCode={shareCode}
                    currentAssistant={currentAssistant}
                    onConfirmDelete={handleDelete}
                    onCancelDelete={closeConfirmDeleteDialog}
                    onSave={saveAssistant}
                    onAssistantUpdated={updateAssistantInfo}
                    onImportAssistant={importAssistant}
                    onCloseUpdateForm={closeUpdateFormDialog}
                    onCloseShare={closeShareDialog}
                    onCloseImport={closeImportDialog}
                />
            </>
        );
    }

    return (
        <>
            <ConfigPageLayout
                sidebar={sidebar}
                content={
                    currentAssistant ? (
                        <AssistantFormRenderer
                            currentAssistant={currentAssistant}
                            formConfig={formConfig}
                            form={form}
                            assistantConfigApi={assistantConfigApi}
                            onSave={handleAssistantFormSave}
                            onCopy={currentAssistant.assistant.id === 1 ? undefined : copyAssistant}
                            onDelete={currentAssistant.assistant.id === 1 ? undefined : openConfirmDeleteDialog}
                            onEdit={openUpdateFormDialog}
                            onShare={handleShareAssistant}
                        />
                    ) : (
                        <EmptyState
                            icon={<Settings className="h-8 w-8 text-muted-foreground" />}
                            title="选择一个助手"
                            description="从左侧列表中选择一个助手开始配置"
                        />
                    )
                }
                selectOptions={selectOptions}
                selectedOptionId={currentAssistant?.assistant.id.toString()}
                onSelectOption={handleSelectFromDropdown}
                selectPlaceholder="选择助手"
                addButton={addButton}
            />

            <AssistantDialogs
                dialogStates={dialogStates}
                shareCode={shareCode}
                currentAssistant={currentAssistant}
                onConfirmDelete={handleDelete}
                onCancelDelete={closeConfirmDeleteDialog}
                onSave={saveAssistant}
                onAssistantUpdated={updateAssistantInfo}
                onImportAssistant={importAssistant}
                onCloseUpdateForm={closeUpdateFormDialog}
                onCloseShare={closeShareDialog}
                onCloseImport={closeImportDialog}
            />
        </>
    );
};

export default AssistantConfig;
