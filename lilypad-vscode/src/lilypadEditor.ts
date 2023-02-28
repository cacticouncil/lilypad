import * as vscode from 'vscode';

export class LilypadEditorProvider implements vscode.CustomTextEditorProvider {

    private static readonly viewType = 'lilypad.frameBased';

    public static register(context: vscode.ExtensionContext): vscode.Disposable {
        const provider = new LilypadEditorProvider(context);
        const providerRegistration = vscode.window.registerCustomEditorProvider(LilypadEditorProvider.viewType, provider);
        return providerRegistration;
    }

    constructor(
		private readonly context: vscode.ExtensionContext
	) { }

    public async resolveCustomTextEditor(
        document: vscode.TextDocument,
        webviewPanel: vscode.WebviewPanel,
        token: vscode.CancellationToken
    ): Promise<void> {
        		// Setup initial content for the webview
		webviewPanel.webview.options = {
			enableScripts: true,
		};
		webviewPanel.webview.html = this.getHtml(webviewPanel.webview);

		function updateWebview() {
			webviewPanel.webview.postMessage({
				type: 'update',
				text: document.getText(),
			});
		}

		// Hook up event handlers so that we can synchronize the webview with the text document.
		//
		// The text document acts as our model, so we have to sync change in the document to our
		// editor and sync changes in the editor back to the document.
		// 
		// Remember that a single text document can also be shared between multiple custom
		// editors (this happens for example when you split a custom editor)

		const changeDocumentSubscription = vscode.workspace.onDidChangeTextDocument(e => {
			if (e.document.uri.toString() === document.uri.toString()) {
				updateWebview();
			}
		});

		// Make sure we get rid of the listener when our editor is closed.
		webviewPanel.onDidDispose(() => {
			changeDocumentSubscription.dispose();
		});

		// Receive message from the webview.
		webviewPanel.webview.onDidReceiveMessage(e => {
			console.log(e);
		});

		updateWebview();
    }

    private getHtml(webview: vscode.Webview): string {
        const scriptUri = webview.asWebviewUri(vscode.Uri.joinPath(
			this.context.extensionUri, 'static', 'run.js'));
        console.log(scriptUri);

        return `
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <title>Druid web example</title>
                <style>
                    html,
                    body,
                    canvas {
                        margin: 0px;
                        padding: 0px;
                        width: 100%;
                        height: 100%;
                        overflow: hidden;
                    }
                </style>
            </head>
            <body>
                <div style="text-align: center; margin: 0px; padding: 0px;">
                    <canvas id="canvas"></canvas>
                </div>
                <script type="module" src="${scriptUri}" type="application/javascript"></script>
            </body>
        </html>
        `;
    }

}
