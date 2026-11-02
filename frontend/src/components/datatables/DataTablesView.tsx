import React, { useState } from 'react';
import { useWorkspace } from '../../context/WorkspaceContext';
import { Database, HardDrive, FileText, Upload, RefreshCw, Layers } from 'lucide-react';
import { CleanRoomStubNotice } from '../common/CleanRoomStubNotice';

export const DataTablesView: React.FC = () => {
  const { currentWorkspace } = useWorkspace();
  const [activeTab, setActiveTab] = useState<'tables' | 'migrations' | 'storage'>('tables');

  return (
    <div className="flex flex-col h-[calc(100vh-3.5rem)] overflow-y-auto p-6">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-xl font-bold text-slate-100 flex items-center gap-2">
            <Database className="w-5 h-5 text-brand-500" />
            <span>Data Tables & Workspace Storage</span>
          </h1>
          <p className="text-xs text-slate-400 mt-1">
            Workspace SQL tables, DuckLake analytical namespaces, migrations, and S3 file storage.
          </p>
        </div>

        <div className="flex items-center gap-2 bg-slate-900 border border-slate-800 p-1 rounded-lg">
          <button
            onClick={() => setActiveTab('tables')}
            className={`px-3 py-1.5 rounded text-xs font-medium transition-colors ${
              activeTab === 'tables' ? 'bg-slate-800 text-slate-100' : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            SQL Tables
          </button>
          <button
            onClick={() => setActiveTab('migrations')}
            className={`px-3 py-1.5 rounded text-xs font-medium transition-colors ${
              activeTab === 'migrations' ? 'bg-slate-800 text-slate-100' : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            Migrations
          </button>
          <button
            onClick={() => setActiveTab('storage')}
            className={`px-3 py-1.5 rounded text-xs font-medium transition-colors ${
              activeTab === 'storage' ? 'bg-slate-800 text-slate-100' : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            S3 Storage
          </button>
        </div>
      </div>

      <CleanRoomStubNotice
        featureName="Data Table Enterprise Roles & Storage Proxies"
        notes="Data table IAM RDS authentication, enterprise DuckLake maintenance, and Azure storage proxies are proprietary upstream modules. Standard PostgreSQL tables and S3-compatible storage helpers run on Gantry's open-source routes."
      />

      {activeTab === 'tables' && (
        <div className="space-y-4">
          <div className="flex justify-between items-center">
            <h2 className="text-sm font-semibold text-slate-200">Workspace Data Tables (PostgreSQL)</h2>
          </div>

          <div className="border border-slate-800 rounded-xl overflow-hidden bg-slate-900/40">
            <table className="w-full text-left text-xs">
              <thead className="bg-slate-900/80 border-b border-slate-800 text-slate-400 font-mono">
                <tr>
                  <th className="p-3">Table Name</th>
                  <th className="p-3">Columns</th>
                  <th className="p-3">Primary Key</th>
                  <th className="p-3">Row Count (approx)</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800/60 font-mono text-slate-300">
                <tr className="hover:bg-slate-900/50">
                  <td className="p-3 font-semibold text-brand-400">users_audit_log</td>
                  <td className="p-3">id, user_id, action, timestamp</td>
                  <td className="p-3">id (UUID)</td>
                  <td className="p-3">142,500</td>
                </tr>
                <tr className="hover:bg-slate-900/50">
                  <td className="p-3 font-semibold text-brand-400">customer_subscriptions</td>
                  <td className="p-3">id, customer_email, tier, status</td>
                  <td className="p-3">id (INT8)</td>
                  <td className="p-3">8,420</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      )}

      {activeTab === 'migrations' && (
        <div className="space-y-4">
          <div className="flex justify-between items-center">
            <h2 className="text-sm font-semibold text-slate-200">Database Migrations</h2>
            <button className="flex items-center gap-1.5 bg-brand-600 hover:bg-brand-500 text-slate-950 font-semibold px-3 py-1.5 rounded-lg text-xs shadow transition-all">
              <RefreshCw className="w-3.5 h-3.5" />
              <span>Apply Pending Migrations</span>
            </button>
          </div>

          <div className="border border-slate-800 rounded-xl overflow-hidden bg-slate-900/40 p-4 font-mono text-xs space-y-2">
            <div className="flex items-center justify-between text-emerald-400">
              <span>✓ 20261028_init_customer_tables.sql</span>
              <span className="text-[11px] text-slate-500">Applied 2026-10-28T14:20:00Z</span>
            </div>
            <div className="flex items-center justify-between text-emerald-400">
              <span>✓ 20261101_add_subscription_indices.sql</span>
              <span className="text-[11px] text-slate-500">Applied 2026-11-01T09:15:00Z</span>
            </div>
          </div>
        </div>
      )}

      {activeTab === 'storage' && (
        <div className="space-y-4">
          <div className="flex justify-between items-center">
            <h2 className="text-sm font-semibold text-slate-200">S3 Workspace Bucket Browser</h2>
            <button className="flex items-center gap-1.5 bg-brand-600 hover:bg-brand-500 text-slate-950 font-semibold px-3 py-1.5 rounded-lg text-xs shadow transition-all">
              <Upload className="w-3.5 h-3.5" />
              <span>Upload File</span>
            </button>
          </div>

          <div className="border border-slate-800 rounded-xl overflow-hidden bg-slate-900/40 p-4 font-mono text-xs">
            <div className="text-slate-400 mb-3">Bucket: <span className="text-slate-200 font-bold">gantry-workspace-{currentWorkspace}</span></div>
            <div className="divide-y divide-slate-800/60">
              <div className="py-2.5 flex items-center justify-between hover:bg-slate-900/50 px-2 rounded">
                <span className="text-slate-200">customers_export_20261101.parquet</span>
                <span className="text-slate-400 text-[11px]">14.2 MB</span>
              </div>
              <div className="py-2.5 flex items-center justify-between hover:bg-slate-900/50 px-2 rounded">
                <span className="text-slate-200">invoices_october.csv</span>
                <span className="text-slate-400 text-[11px]">3.1 MB</span>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
