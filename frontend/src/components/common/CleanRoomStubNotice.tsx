import React from 'react';
import { AlertCircle, ExternalLink } from 'lucide-react';

interface CleanRoomStubNoticeProps {
  featureName: string;
  category?: string;
  notes?: string;
}

export const CleanRoomStubNotice: React.FC<CleanRoomStubNoticeProps> = ({
  featureName,
  category = 'Clean-Room Specification Backlog',
  notes
}) => {
  return (
    <div className="bg-amber-950/40 border border-amber-800/60 rounded-xl p-4 my-4 flex items-start gap-3 text-amber-200">
      <AlertCircle className="w-5 h-5 text-amber-400 shrink-0 mt-0.5" />
      <div className="text-sm space-y-1">
        <div className="font-semibold flex items-center gap-2 text-amber-300">
          <span>{featureName}</span>
          <span className="text-xs px-2 py-0.5 rounded bg-amber-900/60 border border-amber-700/50 font-normal">
            {category}
          </span>
        </div>
        <p className="text-amber-200/90 text-xs leading-relaxed">
          {notes || 
            `In upstream releases, this feature is gated under proprietary licenses. In Gantry, it is excluded or replaced with an open-source stand-in. A clean-room implementation is specified in specs/not-implemented.md.`}
        </p>
      </div>
    </div>
  );
};
