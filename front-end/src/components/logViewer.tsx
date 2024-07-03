import type React from "react";
import { useMemo, useState } from "react";

import { ErrorBoundary } from "@/components/errorBoundary";
import { type ClientLogMessage } from "@/lib/clientLogMessage";

interface LogViewerProps {
    logs: ClientLogMessage[];
    searchTerm: string;
}

export const LogViewer: React.FC<LogViewerProps> = ({
    logs: rawLogs,
    searchTerm,
}) => {
    const MAX_CHARACTERS = 500; // Maximum characters to display for each log message

    const [selectedLog, setSelectedLog] = useState<ClientLogMessage | null>(
        null,
    );

    const logs = useMemo(() => {
        if (!rawLogs) {
            return [];
        }

        if (searchTerm === "") {
            return rawLogs;
        }

        return rawLogs.filter((log: { message: string }) => {
            return log.message.toLowerCase().includes(searchTerm.toLowerCase());
        });
    }, [rawLogs, searchTerm]);

    const truncateMessage = useMemo(() => {
        return (message: string) => {
            return message.length > MAX_CHARACTERS
                ? `${message.slice(0, MAX_CHARACTERS)}...`
                : message;
        };
    }, [MAX_CHARACTERS]);

    const openFullLog: (log: ClientLogMessage) => void = (log) => {
        setSelectedLog(log);
    };

    const closeFullLog: () => void = () => {
        setSelectedLog(null);
    };

    return (
        <div className="flex flex-col h-screen p-4 bg-gray-100">
            {logs.toReversed().map((log) => {
                return (
                    <ErrorBoundary key={log.key}>
                        <div
                            className="flex items-start border-b border-gray-300 py-2 hover:bg-blue-100 cursor-pointer transition-all"
                            onClick={() => {
                                openFullLog(log);
                            }}
                        >
                            <div className="w-1/4 pr-4">
                                <span className="text-gray-500">
                                    {log.timestamp.toLocaleString()}
                                </span>
                            </div>
                            <div className="w-3/4">
                                <pre className="text-gray-700 whitespace-pre-wrap break-all">
                                    {truncateMessage(log.message)}
                                </pre>
                            </div>
                        </div>
                    </ErrorBoundary>
                );
            })}

            {selectedLog && (
                <div className="fixed top-0 right-0 bottom-0 bg-white w-1/2 p-4 overflow-hidden shadow-lg">
                    <div className="flex justify-between items-center mb-4">
                        <h2 className="text-xl font-bold">Full Log</h2>
                        <div>
                            <button
                                className="text-blue-500"
                                onClick={closeFullLog}
                            >
                                Close
                            </button>
                            <button
                                className="bg-blue-500 text-white py-2 px-4 ml-2"
                                onClick={() => {
                                    navigator.clipboard
                                        .writeText(
                                            JSON.stringify(
                                                selectedLog.message,
                                                null,
                                                2,
                                            ),
                                        )
                                        .then(() => {
                                            alert("Copied to clipboard!");
                                        })
                                        .catch((reason: unknown) => {
                                            console.error(reason);

                                            alert(
                                                "Failure to copy to clipboard, check console for error.",
                                            );
                                        });
                                }}
                            >
                                Copy
                            </button>
                        </div>
                    </div>
                    <div className="max-h-screen overflow-y-auto rounded">
                        <h3>Log message</h3>
                        <pre className="bg-gray-800 text-white whitespace-pre-wrap break-all p-4 my-4">
                            {selectedLog.message}
                        </pre>
                        <h3>Timestamp</h3>
                        <div className="bg-gray-800 text-white whitespace-pre-wrap break-all p-4 my-4">
                            <div>{selectedLog.timestamp.toLocaleString()}</div>
                            <div>{selectedLog.timestamp.toString()}</div>
                            <div>{selectedLog.timestamp.toUTCString()}</div>
                            <div>{selectedLog.timestamp.toISOString()}</div>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
};
