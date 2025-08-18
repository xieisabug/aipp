import React, { useCallback, useEffect, useState, useMemo } from 'react';
import { invoke } from "@tauri-apps/api/core";
import LLMProviderConfigForm from "./LLMProviderConfigForm";
import FormDialog from "../FormDialog";
import CustomSelect from "../CustomSelect";
import ConfirmDialog from "../ConfirmDialog";
import ShareDialog from "../ShareDialog";
import ImportDialog from "../ImportDialog";
import PasswordDialog from "../PasswordDialog";
import { Button } from "../ui/button";
import { PlusCircle, Zap, Settings, ServerCrash, Download } from "lucide-react";
import { toast } from 'sonner';

// 导入公共组件
import {
    ConfigPageLayout,
    SidebarList,
    ListItemButton,
    EmptyState,
    SelectOption
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
        { value: 'deepseek', label: 'DeepSeek API' },
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

    // 分享和导入相关状态
    const [shareDialogOpen, setShareDialogOpen] = useState(false);
    const [importDialogOpen, setImportDialogOpen] = useState(false);
    const [passwordDialogOpen, setPasswordDialogOpen] = useState(false);
    const [shareCode, setShareCode] = useState('');

    // 分享提供商
    const handleShareProvider = useCallback(async () => {
        if (!selectedProvider) return;
        setPasswordDialogOpen(true);
    }, [selectedProvider]);

    // 确认导出（设置密码后）
    const handleConfirmExport = useCallback(async (password: string) => {
        if (!selectedProvider) return;
        
        try {
            const code = await invoke<string>('export_llm_provider', { 
                providerId: selectedProvider.id,
                password
            });
            setShareCode(code);
            setShareDialogOpen(true);
        } catch (error) {
            toast.error('分享失败: ' + error);
            throw error; // 重新抛出错误给PasswordDialog处理
        }
    }, [selectedProvider]);

    // 导入提供商
    const handleImportProvider = useCallback(async (
        shareCode: string, 
        password?: string, 
        newName?: string
    ) => {
        if (!password) {
            throw new Error('请输入密码');
        }
        
        await invoke('import_llm_provider', {
            shareCode,
            password,
            newName
        });
        
        // 导入成功后重新获取提供商列表
        getLLMProviderList();
    }, [getLLMProviderList]);

    // 关闭分享对话框
    const closeShareDialog = useCallback(() => {
        setShareDialogOpen(false);
        setShareCode('');
    }, []);

    // 关闭导入对话框
    const closeImportDialog = useCallback(() => {
        setImportDialogOpen(false);
    }, []);

    // 关闭密码对话框
    const closePasswordDialog = useCallback(() => {
        setPasswordDialogOpen(false);
    }, []);

    // 选择提供商
    const handleSelectProvider = useCallback((provider: LLMProvider) => {
        setSelectedProvider(provider);
    }, []);

    // 下拉菜单选项
    const selectOptions: SelectOption[] = useMemo(() =>
        LLMProviders.map(provider => ({
            id: provider.id,
            label: provider.name,
            icon: provider.is_enabled ? <Zap className="h-4 w-4" /> : <Settings className="h-4 w-4" />
        })), [LLMProviders]);

    // 下拉菜单选择回调
    const handleSelectFromDropdown = useCallback((providerId: string) => {
        const provider = LLMProviders.find(p => p.id === providerId);
        if (provider) {
            handleSelectProvider(provider);
        }
    }, [LLMProviders, handleSelectProvider]);

    // 新增按钮组件
    const addButton = useMemo(() => (
        <div className="flex gap-2">
            <Button
                onClick={openNewProviderDialog}
                className="gap-2 bg-primary hover:bg-primary/90 text-primary-foreground shadow-sm hover:shadow-md transition-all"
            >
                <PlusCircle className="h-4 w-4" />
            </Button>
            <Button
                variant="outline"
                onClick={() => setImportDialogOpen(true)}
                className="shadow-sm hover:shadow-md transition-all"
            >
                <Download className="h-4 w-4" />
            </Button>
        </div>
    ), [openNewProviderDialog]);

    // 空状态
    if (LLMProviders.length === 0) {
        return (
            <>
                <ConfigPageLayout
                    sidebar={null}
                    content={
                        <EmptyState
                            icon={<ServerCrash className="h-8 w-8 text-muted-foreground" />}
                            title="还没有配置提供商"
                            description="开始添加你的第一个 AI 模型提供商，享受智能助手的强大功能"
                            action={
                                <div className="flex flex-col gap-3">
                                    <div className="flex gap-2 justify-center">
                                        <Button
                                            onClick={openNewProviderDialog}
                                            className="gap-2 bg-primary hover:bg-primary/90 text-primary-foreground shadow-lg hover:shadow-xl transition-all"
                                        >
                                            <PlusCircle className="h-4 w-4" />
                                            添加第一个提供商
                                        </Button>
                                        <Button
                                            variant="outline"
                                            onClick={() => setImportDialogOpen(true)}
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
                
                {/* 导入对话框 */}
                <ImportDialog
                    title="提供商配置"
                    isOpen={importDialogOpen}
                    requiresPassword={true}
                    onClose={closeImportDialog}
                    onImport={handleImportProvider}
                />
            </>
        );
    }

    // 侧边栏内容
    const sidebar = (
        <SidebarList
            title="提供商"
            description="选择提供商进行配置"
            icon={<ServerCrash className="h-5 w-5" />}
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
                        </div>
                        {provider.is_enabled && (
                            <Zap className="h-3 w-3 ml-2 flex-shrink-0" />
                        )}
                    </div>
                </ListItemButton>
            ))}
        </SidebarList>
    );

    const selectedProviderApiType = selectedProvider ? apiTypes.find(type => type.value === selectedProvider.api_type)?.label || selectedProvider.api_type : "";
    // 右侧内容
    const content = selectedProvider ? (
        <div className="space-y-6">
            <LLMProviderConfigForm
                id={selectedProvider.id}
                index={LLMProviders.findIndex(p => p.id === selectedProvider.id)}
                apiType={selectedProviderApiType}
                name={selectedProvider.name}
                description={selectedProvider.description || `${selectedProviderApiType} 提供商配置`}
                isOffical={selectedProvider.is_official}
                enabled={selectedProvider.is_enabled}
                onToggleEnabled={handleToggle}
                onDelete={() => openConfirmDialog(selectedProvider.id)}
                onShare={handleShareProvider}
            />
        </div>
    ) : (
        <EmptyState
            icon={<Settings className="h-8 w-8 text-muted-foreground" />}
            title="选择一个提供商"
            description="从左侧列表中选择一个提供商开始配置"
        />
    );

    return (
        <>
            <ConfigPageLayout
                sidebar={sidebar}
                content={content}
                selectOptions={selectOptions}
                selectedOptionId={selectedProvider?.id}
                onSelectOption={handleSelectFromDropdown}
                selectPlaceholder="选择提供商"
                addButton={addButton}
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
                        <label className="text-sm font-medium text-foreground">提供商名称</label>
                        <input
                            className="w-full px-3 py-2 border border-input rounded-lg focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring transition-colors bg-background text-foreground"
                            type="text"
                            placeholder="例如：我的 OpenAI"
                            value={providerName}
                            onChange={e => setProviderName(e.target.value)}
                        />
                    </div>
                    <div className="space-y-2">
                        <label className="text-sm font-medium text-foreground">API 调用类型</label>
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

            {/* 密码设置对话框 */}
            <PasswordDialog
                title={selectedProvider?.name || "提供商"}
                isOpen={passwordDialogOpen}
                onClose={closePasswordDialog}
                onConfirm={handleConfirmExport}
            />

            {/* 分享对话框 */}
            <ShareDialog
                title="提供商配置"
                shareCode={shareCode}
                isOpen={shareDialogOpen}
                onClose={closeShareDialog}
            />

            {/* 导入对话框 */}
            <ImportDialog
                title="提供商配置"
                isOpen={importDialogOpen}
                requiresPassword={true}
                onClose={closeImportDialog}
                onImport={handleImportProvider}
            />
        </>
    );
}

export default LLMProviderConfig;
