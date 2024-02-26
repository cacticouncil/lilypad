import pgPromise from 'pg-promise';
import { QueryFile, IQueryFileOptions } from 'pg-promise';
import { join } from 'path';
import dotenv from 'dotenv';

function sqlFile(file: string): QueryFile {
    const fullPath: string = join(__dirname, file);

    const options: IQueryFileOptions = {
        minify: true
    };

    const qf: QueryFile = new QueryFile(fullPath, options);

    if (qf.error) {
        console.error(qf.error);
    }

    return qf;
}

const pg = pgPromise();

dotenv.config();
const db_info = {
    host: process.env.DB_HOST,
    user: process.env.DB_USER,
    database: process.env.DB_NAME,
    password: process.env.DB_PASSWORD,
    port: parseInt(process.env.DB_PORT || "5432")
};

const db = pg(db_info);

const createTables = sqlFile("../sql/createTables.sql");
const insertEvent = sqlFile("../sql/insertEvent.sql");
const insertError = sqlFile("../sql/insertError.sql");

export { db, createTables, insertEvent, insertError };
