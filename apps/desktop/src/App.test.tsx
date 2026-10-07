import { it, expect, afterEach } from "vitest";
import { render, screen, cleanup, fireEvent } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import { DemoWorkspace } from "./DemoWorkspace";
afterEach(cleanup);
it("browser shell never pretends to have a native backend", () => {
  render(
    <QueryClientProvider client={new QueryClient()}>
      <DemoWorkspace />
    </QueryClientProvider>,
  );
  expect(screen.getByText(/Các thao tác cần Rust đã khóa/)).toBeInTheDocument();
  expect(
    screen.getByRole("button", { name: "Lập phương án →" }),
  ).toBeDisabled();
  expect(screen.getByRole("button", { name: "＋ Tạo dự án" })).toBeDisabled();
  expect(
    screen.getByText("Chưa kết nối Rust", { exact: false }),
  ).toBeInTheDocument();
});

it("opens connection and diagnostics before project and does not silently enter demo", () => {
  render(<QueryClientProvider client={new QueryClient()}><App /></QueryClientProvider>);
  expect(screen.getByRole("heading", { name: "Bạn muốn thiết kế gì?" })).toBeInTheDocument();
  expect(screen.queryByText("Thông số tấm đế")).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Kết nối" }));
  expect(screen.getByRole("button", { name: "Continue with ChatGPT" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "Chọn Demo mô phỏng" })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "Chẩn đoán" }));
  expect(screen.getByText("Build ID")).toBeInTheDocument();
});
