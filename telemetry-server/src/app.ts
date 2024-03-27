import express, { NextFunction, Request, Response } from "express";
import dotenv from "dotenv";
import { db, createTables, insertEvent, insertError } from "./db";

dotenv.config();

const app = express();
app.use(express.json());

db.none(createTables);

app.post("/event", (req: Request, res: Response, next: NextFunction) => {
    // info is all properties in body.data that don't contain `common.` in the property name
    const info = Object.keys(req.body.data).reduce((result, key) => {
        if (!key.startsWith("common.")) {
            result[key] = req.body.data[key];
        }
        return result;
    }, {} as Record<string, any>);

    const newEvent = {
        category: (req.body.name as string).replace("CactiCouncil.lilypad-vscode/", ""),
        info: info,
        machine_id: req.body.data["common.vscodemachineid"],
        session_id: req.body.data["common.vscodesessionid"],
        ext_version: req.body.data["common.extversion"],
        vscode_version: req.body.data["common.vscodeversion"]
    }

    db.none(insertEvent, newEvent);

    res.status(200).send("Logged event");
});

app.post("/error", (req: Request, res: Response, next: NextFunction) => {
    const newError = {
        msg: req.body.message,
        machine_id: req.body.data["common.vscodemachineid"],
        session_id: req.body.data["common.vscodesessionid"],
        ext_version: req.body.data["common.extversion"],
        vscode_version: req.body.data["common.vscodeversion"]
    }

    db.none(insertError, newError);

    res.status(200).send("Logged error");
});

app.listen(process.env.PORT, () => {
    console.log(`Server is running at ${process.env.PORT}`);
});
