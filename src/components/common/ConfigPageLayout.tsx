import React, { useState, useEffect } from 'react';
import StatsCard from './StatsCard';

export interface StatItem {
    title: string;
    value: string | number;
    description: string;
    icon: React.ReactNode;
}

interface ConfigPageLayoutProps {
    stats: StatItem[] | null;
    sidebar: React.ReactNode;
    content: React.ReactNode;
    emptyState?: React.ReactNode;
    showEmptyState?: boolean;
}

const ConfigPageLayout: React.FC<ConfigPageLayoutProps> = ({
    stats,
    sidebar,
    content,
    emptyState,
    showEmptyState = false
}) => {
    const [windowHeight, setWindowHeight] = useState(window.innerHeight);

    useEffect(() => {
        const handleResize = () => {
            setWindowHeight(window.innerHeight);
        };

        window.addEventListener('resize', handleResize);
        
        // 清理事件监听器
        return () => {
            window.removeEventListener('resize', handleResize);
        };
    }, []);

    // 只有当窗口高度大于等于800px且stats存在时才显示统计卡片
    const shouldShowStats = stats && windowHeight >= 800;

    return (
        <div className="max-w-7xl mx-auto px-4 py-6 space-y-8">
            {/* 统计卡片 - 根据窗口高度条件性显示 */}
            {shouldShowStats && (
                <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                    {stats.map((stat, index) => (
                        <StatsCard
                            key={index}
                            title={stat.title}
                            value={stat.value}
                            description={stat.description}
                            icon={stat.icon}
                        />
                    ))}
                </div>
            )}

            {/* 主要内容区域 */}
            {showEmptyState ? emptyState : (
                <div className="grid grid-cols-12 gap-6">
                    {/* 左侧列表 */}
                    <div className="col-span-12 lg:col-span-3">
                        {sidebar}
                    </div>

                    {/* 右侧配置区域 */}
                    <div className="col-span-12 lg:col-span-9">
                        {content}
                    </div>
                </div>
            )}
        </div>
    );
};

export default ConfigPageLayout; 