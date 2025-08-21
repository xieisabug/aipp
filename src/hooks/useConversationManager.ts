import { useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Conversation } from '../data/Conversation';

interface DeleteConversationOptions {
  onSuccess?: () => void;
  onError?: (error: Error) => void;
  confirmMessage?: string;
  confirmTitle?: string;
}

function useConversationManager() {
  const deleteConversation = useCallback(async (
    id: string,
    options: DeleteConversationOptions = {}
  ) => {
    const {
      onSuccess,
      onError,
    } = options;

    try {
      await invoke("delete_conversation", { conversationId: +id });

      if (onSuccess) {
        onSuccess();
      }
    } catch (error) {
      if (onError) {
        onError(error as Error);
      } else {
        console.error('Failed to delete conversation:', error);
      }
    }
  }, []);

  const listConversations = useCallback(async (
    page: number = 1,
    pageSize: number = 100
  ): Promise<Conversation[]> => {
    return invoke<Array<Conversation>>("list_conversations", { page, pageSize });
  }, []);

  const forkConversation = useCallback(async (
    conversationId: number,
    messageId: number
  ): Promise<number> => {
    return invoke<number>("fork_conversation", { conversationId, messageId });
  }, []);

  return {
    deleteConversation,
    listConversations,
    forkConversation
  };
}

export default useConversationManager;
