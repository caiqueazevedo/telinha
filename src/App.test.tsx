import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import App from "./App";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("App", () => {
  it("shows the greeting returned by the backend", async () => {
    invoke.mockResolvedValue("Hello, Caique!");
    render(<App />);

    await userEvent.type(screen.getByPlaceholderText("Enter a name..."), "Caique");
    await userEvent.click(screen.getByRole("button", { name: "Greet" }));

    expect(invoke).toHaveBeenCalledWith("greet", { name: "Caique" });
    expect(await screen.findByText("Hello, Caique!")).toBeInTheDocument();
  });
});
