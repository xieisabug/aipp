import React, { useCallback, useEffect, useState, useMemo } from 'react';
import { invoke } from "@tauri-apps/api/core";
import LLMProviderConfigForm from "./LLMProviderConfigForm";
import FormDialog from "../FormDialog";
import CustomSelect from "../CustomSelect";
import ConfirmDialog from "../ConfirmDialog";
import { Button } from "../ui/button";
import { PlusCircle, Zap, Settings, AlertCircle } from "lucide-react";
import { toast } from 'sonner';

// 导入公共组件
import {
    ConfigPageLayout,
    SidebarList,
    ListItemButton,
    InfoCard,
    EmptyState,
    StatItem
} from "../common";

interface LLMProvider {
    id: string;
    name: string;
    api_type: string;
    description: string;
    is_official: boolean;
    is_enabled: boolean;
}

const LLMProviderConfig: React.FC = () => {
    console.log("render llm provider config")
    const [LLMProviders, setLLMProviders] = useState<Array<LLMProvider>>([]);
    const [selectedProvider, setSelectedProvider] = useState<LLMProvider | null>(null);

    const handleToggle = useCallback((index: number) => {
        const newProviders = [...LLMProviders];
        newProviders[index].is_enabled = !newProviders[index].is_enabled;
        setLLMProviders(newProviders);

        invoke('update_llm_provider', {
            id: LLMProviders[index].id,
            name: LLMProviders[index].name,
            apiType: LLMProviders[index].api_type,
            description: LLMProviders[index].description,
            isEnabled: newProviders[index].is_enabled
        });

        // 如果是当前选中的提供商，更新选中状态
        if (selectedProvider && selectedProvider.id === LLMProviders[index].id) {
            setSelectedProvider(newProviders[index]);
        }
    }, [LLMProviders, selectedProvider]);

    const getLLMProviderList = useCallback(() => {
        invoke<Array<LLMProvider>>('get_llm_providers')
            .then((providers) => {
                setLLMProviders(providers);
                // 如果没有选中的提供商，选择第一个
                if (!selectedProvider && providers.length > 0) {
                    setSelectedProvider(providers[0]);
                }
                // 如果当前选中的提供商已被删除，选择第一个
                if (selectedProvider && !providers.find(p => p.id === selectedProvider.id)) {
                    setSelectedProvider(providers.length > 0 ? providers[0] : null);
                }
            })
            .catch((e) => {
                toast.error('获取大模型提供商失败: ' + e);
            });
    }, [selectedProvider]);

    useEffect(() => {
        getLLMProviderList();
    }, []);

    const [newProviderDialogOpen, setNewProviderDialogOpen] = useState(false);
    const [providerName, setProviderName] = useState('');
    const [formApiType, setFormApiType] = useState('openai_api');
    const apiTypes = [
        { value: 'openai_api', label: 'OpenAI API' },
        { value: 'ollama', label: 'Ollama API' },
        { value: 'anthropic', label: 'Anthropic API' },
        { value: 'cohere', label: 'Cohere API' },
    ]

    const openNewProviderDialog = useCallback(() => {
        setNewProviderDialogOpen(true);
    }, []);

    const closeNewProviderDialog = useCallback(() => {
        setNewProviderDialogOpen(false);
    }, []);

    const handleNewProviderSubmit = useCallback(() => {
        invoke('add_llm_provider', {
            name: providerName,
            apiType: formApiType
        }).then(() => {
            toast.success('添加大模型提供商成功');
            setProviderName('');
            setFormApiType('openai_api');
            closeNewProviderDialog();
            getLLMProviderList();
        }).catch((e) => {
            toast.error('添加大模型提供商失败: ' + e);
        });
    }, [providerName, formApiType, closeNewProviderDialog, getLLMProviderList]);

    const [confirmDialogIsOpen, setConfirmDialogIsOpen] = useState(false);
    const [deleteLLMProviderId, setDeleteLLMProviderId] = useState("");

    const onConfirmDeleteProvider = useCallback(() => {
        if (!deleteLLMProviderId) {
            return;
        }
        invoke('delete_llm_provider', { llmProviderId: deleteLLMProviderId }).then(() => {
            toast.success('删除大模型提供商成功');
            getLLMProviderList();
        }).catch(e => {
            toast.error('删除大模型提供商失败: ' + e);
        });
        closeConfirmDialog();
    }, [deleteLLMProviderId, getLLMProviderList]);

    const openConfirmDialog = useCallback((LLMProviderId: string) => {
        setConfirmDialogIsOpen(true)
        setDeleteLLMProviderId(LLMProviderId);
    }, []);

    const closeConfirmDialog = useCallback(() => {
        setConfirmDialogIsOpen(false)
    }, []);

    // 选择提供商
    const handleSelectProvider = useCallback((provider: LLMProvider) => {
        setSelectedProvider(provider);
    }, []);

    // 统计信息
    const stats: StatItem[] = useMemo(() => {
        const enabled = LLMProviders.filter(p => p.is_enabled).length;
        const total = LLMProviders.length;
        const official = LLMProviders.filter(p => p.is_official).length;
        return [
            {
                title: "总提供商",
                value: total,
                description: "已配置的提供商数量",
                icon: <Settings className="h-4 w-4 text-gray-600" />
            },
            {
                title: "已启用",
                value: enabled,
                description: "当前可用的提供商",
                icon: <Zap className="h-4 w-4 text-gray-600" />
            },
            {
                title: "官方支持",
                value: official,
                description: "官方认证的提供商",
                icon: <AlertCircle className="h-4 w-4 text-gray-600" />
            }
        ];
    }, [LLMProviders]);

    // 空状态
    if (LLMProviders.length === 0) {
        return (
            <ConfigPageLayout
                stats={stats}
                sidebar={null}
                content={
                    <EmptyState
                        icon={<Settings className="h-8 w-8 text-gray-500" />}
                        title="还没有配置提供商"
                        description="开始添加你的第一个 AI 模型提供商，享受智能助手的强大功能"
                        action={
                            <Button
                                onClick={openNewProviderDialog}
                                className="gap-2 bg-gray-800 hover:bg-gray-900 text-white shadow-lg hover:shadow-xl transition-all"
                            >
                                <PlusCircle className="h-4 w-4" />
                                添加第一个提供商
                            </Button>
                        }
                    />
                }
            />
        );
    }

    // 侧边栏内容
    const sidebar = (
        <SidebarList
            title="提供商列表"
            description="选择提供商进行配置"
            icon={<Settings className="h-5 w-5" />}
        >
            {LLMProviders.map((provider) => (
                <ListItemButton
                    key={provider.id}
                    isSelected={selectedProvider?.id === provider.id}
                    onClick={() => handleSelectProvider(provider)}
                >
                    <div className="flex items-center w-full">
                        <div className="flex-1 truncate">
                            <div className="font-medium truncate">{provider.name}</div>
                            <div className={`text-xs ${selectedProvider?.id === provider.id ? 'text-gray-300' : 'text-gray-500'}`}>
                                {provider.api_type}
                            </div>
                        </div>
                        {provider.is_enabled && (
                            <Zap className="h-3 w-3 ml-2 flex-shrink-0" />
                        )}
                    </div>
                </ListItemButton>
            ))}
            <div className="pt-2 border-t border-gray-200">
                <Button
                    onClick={openNewProviderDialog}
                    variant="outline"
                    className="w-full gap-2 hover:bg-gray-50 hover:border-gray-300"
                >
                    <PlusCircle className="h-4 w-4" />
                    新增提供商
                </Button>
            </div>
        </SidebarList>
    );

    // 右侧内容
    const content = selectedProvider ? (
        <div className="space-y-6">
            <InfoCard
                icon={<Settings className="h-6 w-6 text-gray-600" />}
                title={selectedProvider.name}
                description={selectedProvider.description || `${selectedProvider.api_type} 提供商配置`}
                badges={[
                    ...(selectedProvider.is_enabled ? [{ text: "已启用", variant: "green" as const }] : []),
                    ...(selectedProvider.is_official ? [{ text: "官方", variant: "blue" as const }] : [])
                ]}
            />
            <LLMProviderConfigForm
                id={selectedProvider.id}
                index={LLMProviders.findIndex(p => p.id === selectedProvider.id)}
                apiType={selectedProvider.api_type}
                name={selectedProvider.name}
                isOffical={selectedProvider.is_official}
                enabled={selectedProvider.is_enabled}
                onToggleEnabled={handleToggle}
                onDelete={openConfirmDialog}
            />
        </div>
    ) : (
        <EmptyState
            icon={<Settings className="h-8 w-8 text-gray-500" />}
            title="选择一个提供商"
            description="从左侧列表中选择一个提供商开始配置"
        />
    );

    return (
        <>
            <ConfigPageLayout
                stats={stats}
                sidebar={sidebar}
                content={content}
            />

            {/* 新增提供商对话框 */}
            <FormDialog
                title='新增大模型提供商'
                isOpen={newProviderDialogOpen}
                onClose={closeNewProviderDialog}
                onSubmit={handleNewProviderSubmit}
            >
                <div className="space-y-5">
                    <div className="space-y-2">
                        <label className="text-sm font-medium text-gray-700">提供商名称</label>
                        <input
                            className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-500 focus:border-gray-500 transition-colors"
                            type="text"
                            placeholder="例如：我的 OpenAI"
                            value={providerName}
                            onChange={e => setProviderName(e.target.value)}
                        />
                    </div>
                    <div className="space-y-2">
                        <label className="text-sm font-medium text-gray-700">API 调用类型</label>
                        <CustomSelect
                            options={apiTypes}
                            value={formApiType}
                            onChange={setFormApiType}
                        />
                    </div>
                </div>
            </FormDialog>

            {/* 确认删除对话框 */}
            <ConfirmDialog
                isOpen={confirmDialogIsOpen}
                title='确认删除'
                confirmText='确定要删除这个提供商吗？删除后相关配置将无法恢复。'
                onConfirm={onConfirmDeleteProvider}
                onCancel={closeConfirmDialog}
            />
        </>
    );
}

export default LLMProviderConfig;
