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
    }, [LLMProviders]);

    const getLLMProviderList = useCallback(() => {
        invoke<Array<LLMProvider>>('get_llm_providers')
            .then(setLLMProviders)
            .catch((e) => {
                toast.error('获取大模型提供商失败: ' + e);
            });
    }, []);

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

    const providerForms = useMemo(() => {
        console.log("provider forms rerender")
        return LLMProviders.map((provider, index) => (
            <LLMProviderConfigForm
                key={provider.id}
                id={provider.id}
                index={index}
                apiType={provider.api_type}
                name={provider.name}
                isOffical={provider.is_official}
                enabled={provider.is_enabled}
                onToggleEnabled={handleToggle}
                onDelete={openConfirmDialog}
            />
        ));
    }, [LLMProviders, handleToggle, openConfirmDialog]);

    // 统计信息
    const stats = useMemo(() => {
        const enabled = LLMProviders.filter(p => p.is_enabled).length;
        const total = LLMProviders.length;
        const official = LLMProviders.filter(p => p.is_official).length;
        return { enabled, total, official };
    }, [LLMProviders]);

    return (
        <div className="max-w-6xl mx-auto px-4 py-6 space-y-8">
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

            {/* 提供商列表 */}
            <div className="space-y-6">
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
                    <div className="space-y-6">
                        {providerForms}
                    </div>
                )}
            </div>

            {/* 添加提供商按钮 */}
            {LLMProviders.length > 0 && (
                <div className="flex justify-center pt-6 pb-4">
                    <Button
                        onClick={openNewProviderDialog}
                        size="lg"
                        className="gap-2 bg-gray-800 hover:bg-gray-900 text-white shadow-lg hover:shadow-xl transition-all transform hover:scale-105"
                    >
                        <PlusCircle className="h-5 w-5" />
                        新增提供商
                    </Button>
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
