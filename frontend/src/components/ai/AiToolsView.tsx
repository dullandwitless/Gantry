import React, { useState } from 'react';
import { useWorkspace } from '../../context/WorkspaceContext';
import { Bot, Terminal, Cpu, CheckCircle2, Copy } from 'lucide-react';
import { useToast } from '../../context/ToastContext';

export const AiToolsView: React.FC = () => {
  const { currentWorkspace } = useWorkspace();
  const { addToast } = useToast();

  const mcpUrl = `${window.location.origin}/mcp/w/${currentWorkspace}/list_tools`;

  const copyMcpUrl = () => {
    navigator.clipboard.writeText(mcpUrl);
    addToast({ kind: 'success', title: 'Copied MCP Endpoint', message: mcpUrl });
  };

  return (
    <div className="flex flex-col h-[calc(100vh-3.5rem)] overflow-y-auto p-6">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-xl font-bold text-slate-100 flex items-center gap-2">
            <Bot className="w-5 h-5 text-brand-500" />
            <span>AI & Model Context Protocol (MCP)</span>
          </h1>
          <p className="text-xs text-slate-400 mt-1">
            Connect AI coding assistants and LLM agents to Gantry runnables via MCP standards.
          </p>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-8">
        {/* MCP Server Configuration */}
        <div className="p-5 rounded-xl border border-slate-800 bg-slate-900/40 space-y-4">
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 rounded-lg bg-purple-950/80 border border-purple-800/80 flex items-center justify-center">
              <Terminal className="w-4 h-4 text-purple-400" />
            </div>
            <div>
              <h2 className="text-sm font-bold text-slate-100">Workspace MCP Server</h2>
              <p className="text-[11px] text-slate-400">Exposes scripts and flows as tools to Claude Desktop, Cursor, etc.</p>
            </div>
          </div>

          <div className="space-y-1">
            <label className="text-[11px] text-slate-400 font-medium">Tool Discovery Endpoint</label>
            <div className="flex items-center gap-2">
              <input
                type="text"
                readOnly
                value={mcpUrl}
                className="flex-1 bg-slate-950 border border-slate-800 rounded px-2.5 py-1.5 text-xs text-slate-300 font-mono focus:outline-none"
              />
              <button
                onClick={copyMcpUrl}
                className="p-1.5 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700"
              >
                <Copy className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>

          <div className="p-3 rounded bg-slate-950 border border-slate-800 text-[11px] font-mono text-slate-400 space-y-1">
            <div className="text-slate-300 font-bold mb-1">Claude Desktop Config (claude_desktop_config.json):</div>
            <pre className="text-purple-300 text-[10px] leading-relaxed overflow-x-auto">
{`{
  "mcpServers": {
    "gantry": {
      "url": "${mcpUrl}"
    }
  }
}`}
            </pre>
          </div>
        </div>

        {/* AI Evaluations */}
        <div className="p-5 rounded-xl border border-slate-800 bg-slate-900/40 space-y-4">
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 rounded-lg bg-brand-950/80 border border-brand-800/80 flex items-center justify-center">
              <Cpu className="w-4 h-4 text-brand-400" />
            </div>
            <div>
              <h2 className="text-sm font-bold text-slate-100">AI Prompt Evaluations</h2>
              <p className="text-[11px] text-slate-400">Benchmark model performance and verify schema output accuracy.</p>
            </div>
          </div>

          <div className="p-4 rounded-lg bg-slate-950 border border-slate-800 text-xs space-y-2">
            <div className="flex items-center justify-between text-slate-300">
              <span className="font-semibold">Customer Intent Classifier</span>
              <span className="text-emerald-400 font-mono text-[11px]">98.5% Accuracy</span>
            </div>
            <div className="text-slate-400 text-[11px]">
              Evaluated 200 synthetic test cases against GPT-4o and Claude 3.5 Sonnet.
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
