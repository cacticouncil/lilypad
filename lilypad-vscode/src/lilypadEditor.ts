import * as vscode from "vscode";

export class LilypadEditorProvider implements vscode.CustomTextEditorProvider {

    private static readonly viewType = "lilypad.frameBased";

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
                type: "update",
                text: document.getText(),
            });
        }

        // Override Clipboard actions
        vscode.commands.registerCommand("editor.action.clipboardCopyAction", _ => {
            webviewPanel.webview.postMessage({
                type: "copy",
            });
        });
        vscode.commands.registerCommand("editor.action.clipboardCutAction", _ => {
            webviewPanel.webview.postMessage({
                type: "cut",
            });
        });
        vscode.commands.registerCommand("editor.action.clipboardPasteAction", _ => {
            vscode.env.clipboard.readText().then(clipboard => {
                webviewPanel.webview.postMessage({
                    type: "paste",
                    text: clipboard
                });
            });
        });

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
        webviewPanel.webview.onDidReceiveMessage(message => {
            switch (message.type) {
                case "started":
                    updateWebview();
                    break;
                case "edited":
                    const range = new vscode.Range(
                        message.range.startLine,
                        message.range.startCol,
                        message.range.endLine,
                        message.range.endCol
                    );
                    this.updateTextDocument(document, message.text, range);
                    break;
                case "set_clipboard":
                    vscode.env.clipboard.writeText(message.text);
                    break;
            }
        });
    }

    private updateTextDocument(document: vscode.TextDocument, newText: string, range: vscode.Range) {
        const edit = new vscode.WorkspaceEdit();
        edit.replace(
            document.uri,
            range,
            newText
        );
        return vscode.workspace.applyEdit(edit);
    }

    private getHtml(webview: vscode.Webview): string {
        const scriptUri = webview.asWebviewUri(vscode.Uri.joinPath(
            this.context.extensionUri, "static", "run.js"));

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
