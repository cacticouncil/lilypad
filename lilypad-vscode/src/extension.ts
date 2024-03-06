import * as vscode from 'vscode';
import { LilypadEditorProvider } from './lilypadEditor';
import { CustomTelemetrySender } from './telemetry';

export let logger: vscode.TelemetryLogger;
export let activeLilypadEditor: vscode.Webview | null;

export function setActiveLilypadEditor(editor: vscode.Webview | null) {
	activeLilypadEditor = editor;
}

export function activate(context: vscode.ExtensionContext) {
	console.log("Lilypad active");

	// register telemetry
	let sender = new CustomTelemetrySender();
	logger = vscode.env.createTelemetryLogger(sender);
	context.subscriptions.push(logger);

	// register custom editor
	context.subscriptions.push(LilypadEditorProvider.register(context));

	// override the undo/redo command within lilypad
	context.subscriptions.push(
		vscode.commands.registerCommand("undo", _ => {
			if (activeLilypadEditor) {
				activeLilypadEditor.postMessage({ type: "undo" });
			} else {
				vscode.commands.executeCommand('default:undo');
			}
		})
	);
	context.subscriptions.push(
		vscode.commands.registerCommand("redo", _ => {
			if (activeLilypadEditor) {
				activeLilypadEditor.postMessage({ type: "redo" });
			} else {
				vscode.commands.executeCommand('default:redo');
			}
		})
	);
}

export function deactivate() { }
