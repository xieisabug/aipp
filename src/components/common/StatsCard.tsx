import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from "../ui/card";

interface StatsCardProps {
    title: string;
    value: string | number;
    description: string;
    icon: React.ReactNode;
}

const StatsCard: React.FC<StatsCardProps> = ({ title, value, description, icon }) => {
    return (
        <Card className="bg-gradient-to-br from-gray-50 to-gray-100 border-gray-200 hover:shadow-md transition-shadow">
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-3">
                <CardTitle className="text-sm font-medium text-gray-700">{title}</CardTitle>
                {icon}
            </CardHeader>
            <CardContent>
                <div className="text-2xl font-bold text-gray-900">{value}</div>
                <p className="text-xs text-gray-600 mt-1">
                    {description}
                </p>
            </CardContent>
        </Card>
    );
};

export default StatsCard; 