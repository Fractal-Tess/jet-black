/* eslint-disable */
/**
 * Generated `api` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type * as auth from "../auth.js";
import type * as http from "../http.js";
import type * as init from "../init.js";
import type * as lib_auth from "../lib/auth.js";
import type * as lib_defaultWorkspace from "../lib/defaultWorkspace.js";
import type * as lib_issues from "../lib/issues.js";
import type * as lib_messages from "../lib/messages.js";
import type * as lib_projects from "../lib/projects.js";
import type * as lib_sampleIssues from "../lib/sampleIssues.js";
import type * as lib_slugs from "../lib/slugs.js";
import type * as lib_states from "../lib/states.js";
import type * as lib_workspaceAccess from "../lib/workspaceAccess.js";
import type * as mutations_accountCleanup from "../mutations/accountCleanup.js";
import type * as mutations_comments from "../mutations/comments.js";
import type * as mutations_issues from "../mutations/issues.js";
import type * as mutations_messages from "../mutations/messages.js";
import type * as mutations_projects from "../mutations/projects.js";
import type * as mutations_workspaces from "../mutations/workspaces.js";
import type * as queries_comments from "../queries/comments.js";
import type * as queries_issues from "../queries/issues.js";
import type * as queries_messages from "../queries/messages.js";
import type * as queries_workspaces from "../queries/workspaces.js";
import type * as schema_issues from "../schema/issues.js";
import type * as schema_messages from "../schema/messages.js";
import type * as schema_workspaces from "../schema/workspaces.js";

import type {
  ApiFromModules,
  FilterApi,
  FunctionReference,
} from "convex/server";

declare const fullApi: ApiFromModules<{
  auth: typeof auth;
  http: typeof http;
  init: typeof init;
  "lib/auth": typeof lib_auth;
  "lib/defaultWorkspace": typeof lib_defaultWorkspace;
  "lib/issues": typeof lib_issues;
  "lib/messages": typeof lib_messages;
  "lib/projects": typeof lib_projects;
  "lib/sampleIssues": typeof lib_sampleIssues;
  "lib/slugs": typeof lib_slugs;
  "lib/states": typeof lib_states;
  "lib/workspaceAccess": typeof lib_workspaceAccess;
  "mutations/accountCleanup": typeof mutations_accountCleanup;
  "mutations/comments": typeof mutations_comments;
  "mutations/issues": typeof mutations_issues;
  "mutations/messages": typeof mutations_messages;
  "mutations/projects": typeof mutations_projects;
  "mutations/workspaces": typeof mutations_workspaces;
  "queries/comments": typeof queries_comments;
  "queries/issues": typeof queries_issues;
  "queries/messages": typeof queries_messages;
  "queries/workspaces": typeof queries_workspaces;
  "schema/issues": typeof schema_issues;
  "schema/messages": typeof schema_messages;
  "schema/workspaces": typeof schema_workspaces;
}>;

/**
 * A utility for referencing Convex functions in your app's public API.
 *
 * Usage:
 * ```js
 * const myFunctionReference = api.myModule.myFunction;
 * ```
 */
export declare const api: FilterApi<
  typeof fullApi,
  FunctionReference<any, "public">
>;

/**
 * A utility for referencing Convex functions in your app's internal API.
 *
 * Usage:
 * ```js
 * const myFunctionReference = internal.myModule.myFunction;
 * ```
 */
export declare const internal: FilterApi<
  typeof fullApi,
  FunctionReference<any, "internal">
>;

export declare const components: {
  betterAuth: import("../betterAuth/_generated/component.js").ComponentApi<"betterAuth">;
};
