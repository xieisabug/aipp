import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export function UserComponent() {
  return (
    <Card className="w-full max-w-md mx-auto">
      <CardHeader>
        <CardTitle>Default Component</CardTitle>
      </CardHeader>
      <CardContent>
        <p className="text-gray-600">
          This is a placeholder component. Replace this file with your actual component code.
        </p>
      </CardContent>
    </Card>
  );
}

export default UserComponent;