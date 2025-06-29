import React from 'react';
import StatsCard from './StatsCard';

export interface StatItem {
    title: string;
    value: string | number;
    description: string;
    icon: React.ReactNode;
}

interface ConfigPageLayoutProps {
    stats: StatItem[];
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
    return (
        <div className="max-w-7xl mx-auto px-4 py-6 space-y-8">
            {/* 统计卡片 */}
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