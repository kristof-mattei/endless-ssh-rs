import type React from "react";
import { useEffect, useState } from "react";

import { io } from "socket.io-client";

import { LogViewer } from "@/components/logViewer";
import { type ClientLogMessage } from "@/lib/clientLogMessage";
import { type ServerLogMessage } from "@/lib/serverLogMessage";

export const App: React.FC = () => {
    const [logs, setLogs] = useState<ClientLogMessage[]>([]);
    const [searchTerm, setSearchTerm] = useState<string>("");

    useEffect(() => {
        const socket = io();

        function parseAndRenderLog(serverLogMessage: ServerLogMessage): void {
            setLogs((previousLogs: ClientLogMessage[]) => {
                return [
                    ...previousLogs,
                    {
                        key: crypto.randomUUID(),
                        timestamp: new Date(serverLogMessage.timestamp),
                        message: serverLogMessage.message,
                    },
                ];
            });
        }

        socket.on("input", parseAndRenderLog);

        return () => {
            // Clean up the socket connection on component unmount
            socket.off("input", parseAndRenderLog);
        };
    }, []); // Empty dependency array ensures the effect runs once on mount

    return (
        <div>
            <div className="flex items-center justify-between bg-gray-800 p-4 text-white">
                <div
                    // vertical div
                    className="flex flex-col items-start justify-center"
                >
                    <a
                        className="text-3xl mb-2"
                        href="https://github.com/kristof-mattei/logscreen"
                        target="_blank"
                        rel="noopener noreferrer"
                    >
                        | logscreen
                    </a>
                    <a
                        className="text-sm"
                        href="https://github.com/sponsors/kristofmattei"
                        target="_blank"
                        rel="noopener noreferrer"
                    >
                        Support this project
                    </a>
                </div>
                <input
                    type="text"
                    placeholder="Search logs..."
                    value={searchTerm}
                    onChange={(e) => {
                        setSearchTerm(e.target.value);
                    }}
                    className="p-2 border border-gray-300 text-gray-800 bg-white rounded w-1/4"
                />
            </div>
            <LogViewer logs={logs} searchTerm={searchTerm} />
        </div>
    );
};
