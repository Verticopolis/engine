import { describe, expect, it } from "vitest";
import { htmlText, removeAll } from "./rustNotices";

describe("removeAll", () => {
  it("removes a match that only forms once an inner one is gone", () => {
    expect(removeAll("a<!<!-- x -->-- y -->b", /<!--[\s\S]*?-->/g)).toBe("ab");
    expect(removeAll("<scr<script>x</script>ipt>y</script>z", /<(script|style)[^>]*>[\s\S]*?<\/\1>/gi)).toBe("z");
  });

  it("leaves text with no match as it is", () => {
    expect(removeAll("plain", /<[^>]*>/g)).toBe("plain");
  });
});

describe("htmlText", () => {
  it("decodes the page's text and drops its markup", () => {
    expect(htmlText("<p>a &#34;b&#34; &#x3c;c&#62; &lt;d&gt; &amp;lt;</p><br>e")).toBe('a "b" <c> <d> &lt;\n\ne');
    expect(htmlText("<!-- a > b -->kept")).toBe("kept");
  });

  it("drops a comment or script split around another one", () => {
    expect(htmlText("<!<!-- x -->-- y -->kept")).toBe("kept");
    expect(htmlText("<scr<script>x</script>ipt>y</script>kept")).toBe("kept");
  });

  it("refuses markup it cannot read rather than passing it on", () => {
    expect(() => htmlText('<a title="x>y">z</a>')).toThrow(/markup/);
    expect(() => htmlText("<p>&copy; 2026</p>")).toThrow(/&copy;/);
  });
});
