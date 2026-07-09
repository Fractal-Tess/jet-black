import { createApi } from "@convex-dev/better-auth";

import { createAuthOptions } from "../auth";
import schema from "./schema";

const api = createApi(schema, createAuthOptions);

export const create: typeof api.create = api.create;
export const findOne: typeof api.findOne = api.findOne;
export const findMany: typeof api.findMany = api.findMany;
export const updateOne: typeof api.updateOne = api.updateOne;
export const updateMany: typeof api.updateMany = api.updateMany;
export const deleteOne: typeof api.deleteOne = api.deleteOne;
export const deleteMany: typeof api.deleteMany = api.deleteMany;
