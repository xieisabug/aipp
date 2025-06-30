import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../ui/card";

interface SidebarListProps {
    title: string;
    description: string;
    icon: React.ReactNode;
    children: React.ReactNode;
    addButton?: React.ReactNode;
}

const SidebarList: React.FC<SidebarListProps> = ({
    title,
    description,
    icon,
    children,
    addButton
}) => {
    return (
        <Card className="bg-gradient-to-br from-gray-50 to-gray-100 border-gray-200 h-fit sticky top-6">
            <CardHeader className="pb-3">
                <div className="flex items-start justify-between">
                    <div className="flex-1 min-w-0">
                        <CardTitle className="text-lg font-semibold text-gray-700 flex items-center gap-2">
                            {icon}
                            {title}
                        </CardTitle>
                        <CardDescription className="text-gray-600 mt-2">
                            {description}
                        </CardDescription>
                    </div>
                    {addButton && (
                        <div className="flex-shrink-0 ml-3">
                            {addButton}
                        </div>
                    )}
                </div>
            </CardHeader>
            <CardContent className="space-y-3">
                {children}
            </CardContent>
        </Card>
    );
};

export default SidebarList; 