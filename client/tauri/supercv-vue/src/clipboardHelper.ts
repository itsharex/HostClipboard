import { invoke } from "@tauri-apps/api/tauri";

export interface ClipboardEntry {
  id: number;
  type: number;
  path: string;
  content: string;
  timestamp: number;
  uuid: string;
}

export class ClipboardHelper {
  static async getClipboardEntries(
    num: number = 10,
    typeList: number[] | null = null
  ): Promise<ClipboardEntry[]> {
    try {
      const result = await invoke<ClipboardEntry[]>(
        "rs_invoke_get_clipboards",
        {
          num,
          typeList,
        }
      );
      return result;
    } catch (error) {
      console.error("Failed to get clipboard entries:", error);
      // throw error;
      return [];
    }
  }

  static async searchClipboardEntries(
    query: string,
    num: number = 10,
    typeList: number[] | null = null
  ): Promise<ClipboardEntry[]> {
    try {
      const result = await invoke<ClipboardEntry[]>(
        "rs_invoke_search_clipboards",
        {
          query,
          num,
          typeList,
        }
      );
      return result;
    } catch (error) {
      console.error("Failed to search clipboard entries:", error);
      // throw error;
      return [];
    }
  }
}
