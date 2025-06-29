import React, { useCallback, useEffect, useState, useMemo } from 'react';
import { invoke } from "@tauri-apps/api/core";
import LLMProviderConfigForm from "./LLMProviderConfigForm";
import FormDialog from "../FormDialog";
import CustomSelect from "../CustomSelect";
import ConfirmDialog from "../ConfirmDialog";
import { Button } from "../ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../ui/card";
import { PlusCircle, Zap, Settings, AlertCircle } from "lucide-react";
import { toast } from 'sonner';

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
    const stats = useMemo(() => {
        const enabled = LLMProviders.filter(p => p.is_enabled).length;
        const total = LLMProviders.length;
        const official = LLMProviders.filter(p => p.is_official).length;
        return { enabled, total, official };
    }, [LLMProviders]);

    return (
        <div className="max-w-7xl mx-auto px-4 py-6 space-y-8">
            {/* 统计卡片 */}
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                <Card className="bg-gradient-to-br from-gray-50 to-gray-100 border-gray-200 hover:shadow-md transition-shadow">
                    <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-3">
                        <CardTitle className="text-sm font-medium text-gray-700">总提供商</CardTitle>
                        <Settings className="h-4 w-4 text-gray-600" />
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-bold text-gray-900">{stats.total}</div>
                        <p className="text-xs text-gray-600 mt-1">
                            已配置的提供商数量
                        </p>
                    </CardContent>
                </Card>

                <Card className="bg-gradient-to-br from-gray-50 to-gray-100 border-gray-200 hover:shadow-md transition-shadow">
                    <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-3">
                        <CardTitle className="text-sm font-medium text-gray-700">已启用</CardTitle>
                        <Zap className="h-4 w-4 text-gray-600" />
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-bold text-gray-900">{stats.enabled}</div>
                        <p className="text-xs text-gray-600 mt-1">
                            当前可用的提供商
                        </p>
                    </CardContent>
                </Card>

                <Card className="bg-gradient-to-br from-gray-50 to-gray-100 border-gray-200 hover:shadow-md transition-shadow">
                    <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-3">
                        <CardTitle className="text-sm font-medium text-gray-700">官方支持</CardTitle>
                        <AlertCircle className="h-4 w-4 text-gray-600" />
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-bold text-gray-900">{stats.official}</div>
                        <p className="text-xs text-gray-600 mt-1">
                            官方认证的提供商
                        </p>
                    </CardContent>
                </Card>
            </div>

            {/* 主要内容区域 */}
            {LLMProviders.length === 0 ? (
                <Card className="border-dashed border-2 border-gray-300 hover:border-gray-400 transition-colors">
                    <CardContent className="flex flex-col items-center justify-center py-16">
                        <div className="w-16 h-16 bg-gray-100 rounded-full flex items-center justify-center mb-4">
                            <Settings className="h-8 w-8 text-gray-500" />
                        </div>
                        <h3 className="text-lg font-semibold text-gray-700 mb-2">
                            还没有配置提供商
                        </h3>
                        <p className="text-gray-500 text-center mb-8 max-w-md leading-relaxed">
                            开始添加你的第一个 AI 模型提供商，享受智能助手的强大功能
                        </p>
                        <Button
                            onClick={openNewProviderDialog}
                            className="gap-2 bg-gray-800 hover:bg-gray-900 text-white shadow-lg hover:shadow-xl transition-all"
                        >
                            <PlusCircle className="h-4 w-4" />
                            添加第一个提供商
                        </Button>
                    </CardContent>
                </Card>
            ) : (
                <div className="grid grid-cols-12 gap-6">
                    {/* 左侧提供商列表 */}
                    <div className="col-span-12 lg:col-span-3">
                        <Card className="bg-gradient-to-br from-gray-50 to-gray-100 border-gray-200 h-fit sticky top-6">
                            <CardHeader className="pb-3">
                                <CardTitle className="text-lg font-semibold text-gray-700 flex items-center gap-2">
                                    <Settings className="h-5 w-5" />
                                    提供商列表
                                </CardTitle>
                                <CardDescription className="text-gray-600">
                                    选择提供商进行配置
                                </CardDescription>
                            </CardHeader>
                            <CardContent className="space-y-3">
                                {LLMProviders.map((provider, index) => (
                                    <Button
                                        key={provider.id}
                                        variant={
                                            selectedProvider?.id === provider.id
                                                ? "default"
                                                : "outline"
                                        }
                                        onClick={() => handleSelectProvider(provider)}
                                        className={`
                                            w-full justify-start text-left transition-all duration-200
                                            ${selectedProvider?.id === provider.id
                                                ? 'bg-gray-800 hover:bg-gray-900 text-white shadow-md'
                                                : 'hover:bg-gray-50 hover:border-gray-300 text-gray-700'
                                            }
                                        `}
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
                                    </Button>
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
                            </CardContent>
                        </Card>
                    </div>

                    {/* 右侧配置区域 */}
                    <div className="col-span-12 lg:col-span-9">
                        {selectedProvider ? (
                            <div className="space-y-6">
                                {/* 提供商信息卡片 */}
                                <Card className="bg-white border-gray-200 shadow-sm">
                                    <CardHeader className="bg-gradient-to-r from-gray-50 to-gray-100 border-b border-gray-200">
                                        <div className="flex items-center justify-between">
                                            <div>
                                                <CardTitle className="text-xl font-bold text-gray-800 flex items-center gap-2">
                                                    <Settings className="h-6 w-6 text-gray-600" />
                                                    {selectedProvider.name}
                                                    {selectedProvider.is_enabled && (
                                                        <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800">
                                                            已启用
                                                        </span>
                                                    )}
                                                    {selectedProvider.is_official && (
                                                        <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800">
                                                            官方
                                                        </span>
                                                    )}
                                                </CardTitle>
                                                <CardDescription className="mt-1 text-gray-600">
                                                    {selectedProvider.description || `${selectedProvider.api_type} 提供商配置`}
                                                </CardDescription>
                                            </div>
                                        </div>
                                    </CardHeader>
                                </Card>

                                {/* 配置表单 */}
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
                            <Card className="border-dashed border-2 border-gray-300">
                                <CardContent className="flex flex-col items-center justify-center py-16">
                                    <div className="w-16 h-16 bg-gray-100 rounded-full flex items-center justify-center mb-4">
                                        <Settings className="h-8 w-8 text-gray-500" />
                                    </div>
                                    <h3 className="text-lg font-semibold text-gray-700 mb-2">
                                        选择一个提供商
                                    </h3>
                                    <p className="text-gray-500 text-center max-w-md leading-relaxed">
                                        从左侧列表中选择一个提供商开始配置
                                    </p>
                                </CardContent>
                            </Card>
                        )}
                    </div>
                </div>
            )}

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
        </div>
    );
}

export default LLMProviderConfig;
