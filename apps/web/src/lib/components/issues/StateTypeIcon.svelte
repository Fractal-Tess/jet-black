<script lang="ts">
import type { IssueState } from "./types";

let {
  type,
  color,
  class: className = "size-3.5",
}: {
  type: IssueState["type"];
  color: string;
  class?: string;
} = $props();
</script>

<svg
  aria-hidden="true"
  class="{className} shrink-0"
  fill="none"
  viewBox="0 0 16 16"
  xmlns="http://www.w3.org/2000/svg"
>
  {#if type === "backlog"}
    <circle
      cx="8"
      cy="8"
      r="6"
      stroke={color}
      stroke-dasharray="3 3"
      stroke-linecap="round"
      stroke-width="1.5"
    />
  {:else if type === "unstarted"}
    <circle cx="8" cy="8" r="6" stroke={color} stroke-width="1.5" />
  {:else if type === "started"}
    <circle cx="8" cy="8" r="6" stroke={color} stroke-width="1.5" />
    <path d="M8 5 A3 3 0 0 1 8 11 Z" fill={color} />
  {:else if type === "completed"}
    <path
      clip-rule="evenodd"
      d="M8 15C11.866 15 15 11.866 15 8C15 4.13401 11.866 1 8 1C4.13401 1 1 4.13401 1 8C1 11.866 4.13401 15 8 15ZM11.3587 6.18828C11.6007 5.85214 11.5244 5.38343 11.1882 5.14141C10.8521 4.89938 10.3834 4.97568 10.1414 5.31183L7.03706 9.62335L5.25956 7.97751C4.95563 7.69609 4.4811 7.71434 4.19968 8.01828C3.91826 8.32221 3.93651 8.79673 4.24045 9.07815L6.64045 11.3004C6.79816 11.4464 7.01095 11.5178 7.22481 11.4963C7.43868 11.4749 7.63307 11.3627 7.75865 11.1883L11.3587 6.18828Z"
      fill={color}
      fill-rule="evenodd"
    />
  {:else}
    <path
      clip-rule="evenodd"
      d="M8 15C11.866 15 15 11.866 15 8C15 4.13401 11.866 1 8 1C4.13401 1 1 4.13401 1 8C1 11.866 4.13401 15 8 15ZM11.1018 4.89826C11.3947 5.19115 11.3947 5.66603 11.1018 5.95892L9.06068 8.00002L11.1018 10.0411C11.3947 10.334 11.3947 10.8089 11.1018 11.1018C10.8089 11.3947 10.334 11.3947 10.0411 11.1018L8.00002 9.06068L5.95892 11.1018C5.66603 11.3947 5.19115 11.3947 4.89826 11.1018C4.60537 10.8089 4.60537 10.334 4.89826 10.0411L6.93936 8.00002L4.89826 5.95892C4.60537 5.66603 4.60537 5.19115 4.89826 4.89826C5.19115 4.60537 5.66603 4.60537 5.95892 4.89826L8.00002 6.93936L10.0411 4.89826C10.334 4.60537 10.8089 4.60537 11.1018 4.89826Z"
      fill={color}
      fill-rule="evenodd"
    />
  {/if}
</svg>
