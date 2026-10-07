import { it, expect, afterEach } from "vitest";
import { cleanup } from "@testing-library/react";
afterEach(cleanup);
import { render, screen } from "@testing-library/react";
import { Preview } from "./Preview";
import { defaultPlate } from "./contracts";
import { errorMessage } from "./ipc";
it("renders bounded primitives with CAD Y axis and units", () => {
  const { container } = render(
    <Preview
      geometry={{
        parameters: defaultPlate,
        entities: [
          {
            kind: "circle",
            id: "h1",
            layer: "AI_HOLES",
            center: [20, 20],
            radius: 5,
          },
        ],
      }}
    />,
  );
  expect(screen.getByRole("img")).toHaveAttribute(
    "aria-label",
    "Preview hình học tấm đế theo mm",
  );
  expect(container.querySelector("circle")).toHaveAttribute("r", "5");
  expect(container.querySelector("g")).toHaveAttribute(
    "transform",
    "scale(1,-1)",
  );
  expect(screen.getByText("200 mm")).toBeInTheDocument();
});
it("shows empty state and structured IPC errors", () => {
  render(<Preview geometry={null} />);
  expect(screen.getByText("Chưa có hình học")).toBeInTheDocument();
  expect(errorMessage({ code: "CONFLICT", message: "Cần lập lại" })).toBe(
    "Cần lập lại",
  );
});
