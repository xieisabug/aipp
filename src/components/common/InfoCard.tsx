import React from 'react';
import { Card, CardDescription, CardHeader, CardTitle } from "../ui/card";

interface StatusBadge {
    text: string;
    variant: 'green' | 'blue' | 'gray';
}

interface InfoCardProps {
    icon: React.ReactNode;
    title: string;
    description: string;
    badges?: StatusBadge[];
    actions?: React.ReactNode;
}

const InfoCard: React.FC<InfoCardProps> = ({
    icon,
    title,
    description,
    badges = [],
    actions
}) => {
    const getBadgeClasses = (variant: StatusBadge['variant']) => {
        switch (variant) {
            case 'green':
                return 'bg-green-100 text-green-800';
            case 'blue':
                return 'bg-blue-100 text-blue-800';
            case 'gray':
            default:
                return 'bg-gray-100 text-gray-800';
        }
    };

    return (
        <Card className="bg-white border-gray-200 shadow-sm">
            <CardHeader className="bg-gradient-to-r from-gray-50 to-gray-100 border-b border-gray-200">
                <div className="flex items-center justify-between">
                    <div>
                        <CardTitle className="text-xl font-bold text-gray-800 flex items-center gap-2">
                            {icon}
                            {title}
                            {badges.map((badge, index) => (
                                <span
                                    key={index}
                                    className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${getBadgeClasses(badge.variant)}`}
                                >
                                    {badge.text}
                                </span>
                            ))}
                        </CardTitle>
                        <CardDescription className="mt-1 text-gray-600">
                            {description}
                        </CardDescription>
                    </div>
                    {actions && (
                        <div className="flex items-center gap-2">
                            {actions}
                        </div>
                    )}
                </div>
            </CardHeader>
        </Card>
    );
};

export default InfoCard; 