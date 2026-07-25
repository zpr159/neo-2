import { describe, it, expect } from "vitest";
import { VERSION, APP_NAME } from "../src/index";

describe("Neo UI", () => {
  it("has correct version", () => {
    expect(VERSION).toBe("0.1.0");
  });

  it("has correct app name", () => {
    expect(APP_NAME).toBe("Neo UI");
  });
});
