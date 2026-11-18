# Building a Streaming Chatbot

The `useChat` hook makes it effortless to create a conversational user interface for your chatbot application. It enables the streaming of chat messages from your AI provider, manages the chat state, and updates the UI automatically as new messages arrive.

To summarize, the `useChat` hook provides the following features:

- **Message Streaming**: All the messages from the AI provider are streamed to the chat UI in real-time.
- **Managed States**: The hook manages the states for input, messages, status, error and more for you.
- **Seamless Integration**: Easily integrate your chat AI into any design or layout with minimal effort.

In this guide, you will learn how to use the `useChat` hook to create a chatbot application with real-time message streaming. Check out our [chatbot with tools guide](/docs/ai-sdk-ui/chatbot-tool-usage) to learn how to use tools in your chatbot.

## Example

The request flow works like this:

1. The user submits a message and `sendMessage` posts it to your API route.
2. Your route calls the provider and returns a UI message stream.
3. The hook appends chunks to the last message as they arrive, re-rendering as it goes.

```
'use client';

import { useChat } from '@ai-sdk/react';

export default function Chat() {
  const { messages, sendMessage, status } = useChat();

  return (
    <>
      {messages.map(message => (
        <Message key={message.id} message={message} />
      ))}
      <ChatInput
        onSubmit={text => sendMessage({ text })}
        disabled={status !== 'ready'}
      />
    </>
  );
}
```

```
import { openai } from '@ai-sdk/openai';
import { convertToModelMessages, streamText } from 'ai';

export async function POST(req: Request) {
  const { messages } = await req.json();

  const result = streamText({
    model: openai('gpt-4o'),
    messages: await convertToModelMessages(messages),
  });

  return result.toUIMessageStreamResponse();
}
```

> The UI messages have a new `parts` property that contains the message parts. We recommend rendering the messages using the `parts` property instead of the `content` property. The parts property supports different message types, including text, tool invocation, and tool result, and allows for more flexible and complex chat UIs.

In the `Page` component, the `useChat` hook will request to your AI provider endpoint whenever the user sends a message using `sendMessage`. The messages are then streamed back in real-time and displayed in the chat UI.

## Customized UI

`useChat` also provides ways to manage the chat message states via code, show status, and update messages without being triggered by user interactions.

### Status

The `useChat` hook returns a `status`. It has the following possible values:

- `submitted`: The message has been sent to the API and we're awaiting the start of the response stream.
- `streaming`: The response is actively streaming in from the API, receiving chunks of data.
- `ready`: The full response has been received and processed; a new user message can be submitted.
- `error`: An error occurred during the API request, preventing successful completion.

```
const { messages, sendMessage, status, stop } = useChat({
  transport: new DefaultChatTransport({ api: '/api/chat' }),
});

// ...

{(status === 'submitted' || status === 'streaming') && (
  <div>
    {status === 'submitted' && <Spinner />}
    <button type="button" onClick={() => stop()}>
      Stop
    </button>
  </div>
)}
```

### Error State

Similarly, the `error` state reflects the error object thrown during the fetch request. It can be used to display an error message, disable the submit button, or show a retry button:

> We recommend showing a generic error message to the user, such as "Something went wrong." This is a good practice to avoid leaking information from the server.

```
const { messages, sendMessage, error, regenerate } = useChat({
  transport: new DefaultChatTransport({ api: '/api/chat' }),
});

// ...

{error && (
  <>
    <div>An error occurred.</div>
    <button type="button" onClick={() => regenerate()}>
      Retry
    </button>
  </>
)}
```

### Cancellation and regeneration

It's also a common use case to abort the response message while it's still streaming back from the AI provider. You can do this by calling the `stop` function returned by the `useChat` hook.

```
const { stop, status } = useChat();

<button
  onClick={stop}
  disabled={!(status === 'streaming' || status === 'submitted')}
>
  Stop
</button>
```

---

## API reference

### useChat(options)

Creates a chat helper. All options are optional; the defaults talk to `/api/chat` and render at native stream speed.

| Prop | Type | Description |
|---|---|---|
| `transport` | `ChatTransport<UIMessage>` | How messages reach your API route |
| `messages` | `UIMessage[]` | Initial messages to seed the conversation |
| `onFinish` | `(event: FinishEvent) => void` | Runs when the assistant response completes |
| `onError` | `(error: Error) => void` | Runs when the fetch request fails |
| `throttle` | `number` | Milliseconds between UI updates while streaming |

## Event Callbacks

`useChat` provides optional event callbacks that you can use to handle different stages of the chatbot lifecycle:

- `onFinish`: Called when the assistant response is completed. The event includes the response message, all messages, and flags for abort, disconnect, and errors.
- `onError`: Called when an error occurs during the fetch request.
- `onData`: Called whenever a data part is received.

These callbacks can be used to trigger additional actions, such as logging, analytics, or custom UI updates.

```
const { messages } = useChat({
  onFinish: ({ message }) => saveToHistory(message),
  onError: error => console.error(error),
});
```

---

## Math

Display math sits in the flow rhythm and scrolls when it runs long. Inline math like <math><msup><mi>e</mi><mrow><mi>i</mi><mi>π</mi></mrow></msup><mo>+</mo><mn>1</mn><mo>=</mo><mn>0</mn></math> rides the line without stretching it.

### Display

The quadratic formula, as a block:

<math display="block">
  <mi>x</mi><mo>=</mo>
  <mfrac>
    <mrow><mo>−</mo><mi>b</mi><mo>±</mo><msqrt><mrow><msup><mi>b</mi><mn>2</mn></msup><mo>−</mo><mn>4</mn><mi>a</mi><mi>c</mi></mrow></msqrt></mrow>
    <mrow><mn>2</mn><mi>a</mi></mrow>
  </mfrac>
</math>

Prose continues after the block at the normal distance, so equations read as part of the argument, not decoration.

### Overflow

A long expansion scrolls inside its own box instead of breaking the column:

<math display="block">
  <msup><mrow><mo>(</mo><mi>a</mi><mo>+</mo><mi>b</mi><mo>)</mo></mrow><mn>4</mn></msup><mo>=</mo>
  <msup><mi>a</mi><mn>4</mn></msup><mo>+</mo>
  <mn>4</mn><msup><mi>a</mi><mn>3</mn></msup><mi>b</mi><mo>+</mo>
  <mn>6</mn><msup><mi>a</mi><mn>2</mn></msup><msup><mi>b</mi><mn>2</mn></msup><mo>+</mo>
  <mn>4</mn><mi>a</mi><msup><mi>b</mi><mn>3</mn></msup><mo>+</mo>
  <msup><mi>b</mi><mn>4</mn></msup>
</math>
