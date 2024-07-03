import { type UUID } from "crypto";

export interface ClientLogMessage {
    key: UUID;
    message: string;
    timestamp: Date;
}
