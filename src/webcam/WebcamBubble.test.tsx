import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { WebcamBubble } from "./WebcamBubble";

function fakeStream() {
  const track = { stop: vi.fn() };
  return { stream: { getTracks: () => [track] } as unknown as MediaStream, track };
}

function stubCamera(getUserMedia: () => Promise<MediaStream>) {
  Object.defineProperty(navigator, "mediaDevices", { value: { getUserMedia: vi.fn(getUserMedia) }, configurable: true });
}

function renderBubble() {
  const handlers = { onDrag: vi.fn(), onClose: vi.fn() };
  const view = render(<WebcamBubble {...handlers} />);
  return { ...handlers, ...view, video: view.container.querySelector("video") as HTMLVideoElement };
}

afterEach(() => {
  Reflect.deleteProperty(navigator, "mediaDevices");
});

describe("WebcamBubble", () => {
  it("shows the camera stream once it starts", async () => {
    const { stream } = fakeStream();
    stubCamera(async () => stream);
    const { video } = renderBubble();

    await waitFor(() => expect(video).toBeVisible());
    expect(video.srcObject).toBe(stream);
    expect(navigator.mediaDevices.getUserMedia).toHaveBeenCalledWith(expect.objectContaining({ audio: false }));
  });

  it("explains a blocked camera", async () => {
    stubCamera(async () => {
      throw new DOMException("denied", "NotAllowedError");
    });
    renderBubble();

    expect(await screen.findByRole("alert")).toHaveTextContent("Câmera bloqueada");
  });

  it("explains a missing camera", async () => {
    stubCamera(async () => {
      throw new DOMException("none", "NotFoundError");
    });
    renderBubble();

    expect(await screen.findByRole("alert")).toHaveTextContent("Nenhuma câmera");
  });

  it("releases the camera when closed", async () => {
    const { stream, track } = fakeStream();
    stubCamera(async () => stream);
    const { video, unmount } = renderBubble();
    await waitFor(() => expect(video).toBeVisible());

    unmount();

    expect(track.stop).toHaveBeenCalled();
  });

  it("drags the window on a left press and not on the close button", async () => {
    stubCamera(() => new Promise(() => {}));
    const { onDrag, onClose } = renderBubble();
    const bubble = screen.getByLabelText("Webcam");

    fireEvent.mouseDown(bubble, { button: 2 });
    expect(onDrag).not.toHaveBeenCalled();
    fireEvent.mouseDown(bubble, { button: 0 });
    expect(onDrag).toHaveBeenCalledTimes(1);

    await userEvent.click(screen.getByRole("button", { name: "Fechar webcam" }));
    expect(onDrag).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalled();
  });
});
