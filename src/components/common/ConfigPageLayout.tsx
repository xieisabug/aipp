import React, { useState, useEffect, cloneElement } from 'react';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/select';

export interface SelectOption {
    id: string;
    label: string;
    icon?: React.ReactNode;
}

interface ConfigPageLayoutProps {
    sidebar: React.ReactNode;
    content: React.ReactNode;
    emptyState?: React.ReactNode;
    showEmptyState?: boolean;
    // 响应式下拉菜单相关props
    selectOptions?: SelectOption[];
    selectedOptionId?: string;
    onSelectOption?: (optionId: string) => void;
    selectPlaceholder?: string;
    addButton?: React.ReactNode;
}

const ConfigPageLayout: React.FC<ConfigPageLayoutProps> = ({
    sidebar,
    content,
    emptyState,
    showEmptyState = false,
    selectOptions,
    selectedOptionId,
    onSelectOption,
    selectPlaceholder = "选择项目",
    addButton,
}) => {
    const [windowWidth, setWindowWidth] = useState(window.innerWidth);

    useEffect(() => {
        const handleResize = () => {
            setWindowWidth(window.innerWidth);
        };

        window.addEventListener('resize', handleResize);
        
        // 清理事件监听器
        return () => {
            window.removeEventListener('resize', handleResize);
        };
    }, []);

    // 小屏幕时使用下拉菜单（宽度小于1200px）
    const isSmallScreen = windowWidth < 1200;
    const shouldShowDropdown = isSmallScreen && selectOptions && selectOptions.length > 0;

    // 为sidebar添加addButton props（如果sidebar是SidebarList组件）
    const enhancedSidebar = sidebar && React.isValidElement(sidebar) && !shouldShowDropdown
        ? cloneElement(sidebar as React.ReactElement<any>, {
            addButton: addButton
        })
        : sidebar;

    const renderDropdownHeader = () => {
        if (!shouldShowDropdown) return null;

        const selectedOption = selectOptions?.find(option => option.id === selectedOptionId);

        return (
            <div className="mb-6">
                <div className="flex items-center gap-3">
                    <div className="flex-1">
                        <Select value={selectedOptionId} onValueChange={onSelectOption}>
                            <SelectTrigger className="w-full">
                                <SelectValue placeholder={selectPlaceholder}>
                                    {selectedOption && (
                                        <div className="flex items-center gap-2">
                                            {selectedOption.icon}
                                            <span>{selectedOption.label}</span>
                                        </div>
                                    )}
                                </SelectValue>
                            </SelectTrigger>
                            <SelectContent>
                                {selectOptions?.map((option) => (
                                    <SelectItem key={option.id} value={option.id}>
                                        <div className="flex items-center gap-2">
                                            {option.icon}
                                            <span>{option.label}</span>
                                        </div>
                                    </SelectItem>
                                ))}
                            </SelectContent>
                        </Select>
                    </div>
                    {addButton && (
                        <div className="flex-shrink-0">
                            {addButton}
                        </div>
                    )}
                </div>
            </div>
        );
    };

    return (
        <div className="max-w-none mx-auto px-4 py-6 space-y-8">
            {/* 响应式下拉菜单 - 小屏幕时显示 */}
            {renderDropdownHeader()}

            {/* 主要内容区域 */}
            {showEmptyState ? emptyState : (
                <div className={`grid gap-6 ${shouldShowDropdown ? 'grid-cols-1' : 'grid-cols-12'}`}>
                    {/* 左侧列表 - 大屏幕时显示 */}
                    {!shouldShowDropdown && (
                        <div className="col-span-12 lg:col-span-4 xl:col-span-4 2xl:col-span-3">
                            {enhancedSidebar}
                        </div>
                    )}

                    {/* 右侧配置区域 */}
                    <div className={shouldShowDropdown ? 'col-span-1' : 'col-span-12 lg:col-span-8 xl:col-span-8 2xl:col-span-9'}>
                        {content}
                    </div>
                </div>
            )}
        </div>
    );
};

export default ConfigPageLayout; 