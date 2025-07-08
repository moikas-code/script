/**
 * MCP Tool implementations for Script KB
 */
import { Tool } from '@modelcontextprotocol/sdk/types.js';
import { KBManager } from './kb-manager.js';
export declare function createTools(_kbManager: KBManager): Tool[];
/**
 * Execute a tool with the given arguments
 */
export declare function executeTool(toolName: string, args: any, kbManager: KBManager): Promise<any>;
//# sourceMappingURL=tools.d.ts.map