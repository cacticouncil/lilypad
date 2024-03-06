import * as vscode from "vscode";
import fetch from "node-fetch";

// set to undefined during development, and the actual server when shipping
const server: string | undefined = undefined;

export class CustomTelemetrySender implements vscode.TelemetrySender {
    sendEventData(eventName: string, data?: Record<string, any> | undefined): void {
        if (server) {
            fetch(`${server}/event`, {
                method: "POST",
                headers: {
                    // eslint-disable-next-line @typescript-eslint/naming-convention
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    name: eventName,
                    data: data
                })
            }).catch((err) => {
                console.error("Lilypad Telemetry Error: " + err.message);
            });
        }

    }

    sendErrorData(error: Error, data?: Record<string, any> | undefined): void {
        if (server) {
            fetch(`${server}/error`, {
                method: "POST",
                headers: {
                    // eslint-disable-next-line @typescript-eslint/naming-convention
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    message: error.message,
                    data: data
                })
            }).catch((err) => {
                console.error("Lilypad Telemetry Error: " + err.message);
            });
        }
    }

}
