export class OutputLimiter {
  private bytes = 0;

  constructor(private readonly maxBytes: number) {}

  accept(text: string) {
    this.bytes += text.length;
    return this.bytes <= this.maxBytes;
  }

  reset() {
    this.bytes = 0;
  }
}
