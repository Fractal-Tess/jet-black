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
import type * as mutations_attachments from "../mutations/attachments.js";
import type * as mutations_comments from "../mutations/comments.js";
import type * as mutations_intake from "../mutations/intake.js";
import type * as mutations_issues from "../mutations/issues.js";
import type * as mutations_labels from "../mutations/labels.js";
import type * as mutations_messages from "../mutations/messages.js";
import type * as mutations_modules from "../mutations/modules.js";
import type * as mutations_pages from "../mutations/pages.js";
import type * as mutations_projects from "../mutations/projects.js";
import type * as mutations_sprints from "../mutations/sprints.js";
import type * as mutations_users from "../mutations/users.js";
import type * as mutations_workspaces from "../mutations/workspaces.js";
import type * as queries_attachments from "../queries/attachments.js";
import type * as queries_comments from "../queries/comments.js";
import type * as queries_dashboard from "../queries/dashboard.js";
import type * as queries_intake from "../queries/intake.js";
import type * as queries_issues from "../queries/issues.js";
import type * as queries_labels from "../queries/labels.js";
import type * as queries_messages from "../queries/messages.js";
import type * as queries_modules from "../queries/modules.js";
import type * as queries_pages from "../queries/pages.js";
import type * as queries_sprints from "../queries/sprints.js";
import type * as queries_workspaces from "../queries/workspaces.js";
import type * as schema_intake from "../schema/intake.js";
import type * as schema_issues from "../schema/issues.js";
import type * as schema_messages from "../schema/messages.js";
import type * as schema_modules from "../schema/modules.js";
import type * as schema_pages from "../schema/pages.js";
import type * as schema_sprints from "../schema/sprints.js";
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
  "mutations/attachments": typeof mutations_attachments;
  "mutations/comments": typeof mutations_comments;
  "mutations/intake": typeof mutations_intake;
  "mutations/issues": typeof mutations_issues;
  "mutations/labels": typeof mutations_labels;
  "mutations/messages": typeof mutations_messages;
  "mutations/modules": typeof mutations_modules;
  "mutations/pages": typeof mutations_pages;
  "mutations/projects": typeof mutations_projects;
  "mutations/sprints": typeof mutations_sprints;
  "mutations/users": typeof mutations_users;
  "mutations/workspaces": typeof mutations_workspaces;
  "queries/attachments": typeof queries_attachments;
  "queries/comments": typeof queries_comments;
  "queries/dashboard": typeof queries_dashboard;
  "queries/intake": typeof queries_intake;
  "queries/issues": typeof queries_issues;
  "queries/labels": typeof queries_labels;
  "queries/messages": typeof queries_messages;
  "queries/modules": typeof queries_modules;
  "queries/pages": typeof queries_pages;
  "queries/sprints": typeof queries_sprints;
  "queries/workspaces": typeof queries_workspaces;
  "schema/intake": typeof schema_intake;
  "schema/issues": typeof schema_issues;
  "schema/messages": typeof schema_messages;
  "schema/modules": typeof schema_modules;
  "schema/pages": typeof schema_pages;
  "schema/sprints": typeof schema_sprints;
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
