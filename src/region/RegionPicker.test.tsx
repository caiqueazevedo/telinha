import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { RegionPicker } from "./RegionPicker";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

function drag(from: [number, number], to: [number, number]) {
  const picker = screen.getByTestId("picker");
  fireEvent.mouseDown(picker, { clientX: from[0], clientY: from[1] });
  fireEvent.mouseMove(picker, { clientX: to[0], clientY: to[1] });
  fireEvent.mouseUp(picker, { clientX: to[0], clientY: to[1] });
}

beforeEach(() => invoke.mockReset());

describe("RegionPicker", () => {
  it("sends the dragged area in physical pixels", () => {
    render(<RegionPicker scale={1.5} />);

    drag([300, 200], [100, 40]);

    expect(invoke).toHaveBeenCalledWith("finish_region_pick", {
      selection: { x: 150, y: 60, width: 300, height: 240 },
    });
  });

  it("shows the size of the area while dragging", () => {
    render(<RegionPicker scale={2} />);
    const picker = screen.getByTestId("picker");

    fireEvent.mouseDown(picker, { clientX: 10, clientY: 10 });
    fireEvent.mouseMove(picker, { clientX: 110, clientY: 60 });

    expect(screen.getByLabelText("Área selecionada")).toHaveTextContent("200 × 100");
  });

  it("ignores a click without a usable drag", () => {
    render(<RegionPicker scale={1} />);

    drag([50, 50], [55, 52]);

    expect(invoke).not.toHaveBeenCalled();
    expect(screen.getByText(/Arraste para selecionar/)).toBeInTheDocument();
  });

  it("cancels on Escape", () => {
    render(<RegionPicker scale={1} />);

    fireEvent.keyDown(window, { key: "Escape" });

    expect(invoke).toHaveBeenCalledWith("finish_region_pick", { selection: null });
  });
});
