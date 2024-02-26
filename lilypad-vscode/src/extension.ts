import * as vscode from 'vscode';
import { LilypadEditorProvider } from './lilypadEditor';
import { CustomTelemetrySender } from './telemetry';

const key = '';
export let logger: vscode.TelemetryLogger;

export function activate(context: vscode.ExtensionContext) {
	console.log('Extension active');

	// register telemetry
	let sender = new CustomTelemetrySender();
	logger = vscode.env.createTelemetryLogger(sender);
	context.subscriptions.push(logger);

	// register custom editor
	context.subscriptions.push(LilypadEditorProvider.register(context));
}

export function deactivate() { }
