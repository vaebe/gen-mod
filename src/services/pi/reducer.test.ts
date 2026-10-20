import { expect, it } from 'vitest'
import { emptyChat, reduceRecord, restoreChat } from './reducer'

const memorySave = { outcome: 'saved', noteId: 'memory-1', revision: 2, title: '版本适配' }
const content = [{ type: 'text', text: '原生已提交' }]

it('实时和历史保留相同的已提交记忆回执', () => {
  const live = reduceRecord(emptyChat(), { type: 'tool_execution_end', toolCallId: 'call', toolName: 'save_long_term_memory', result: { content, details: { memorySave } } })
  const history = restoreChat([{ role: 'toolResult', toolCallId: 'call', toolName: 'save_long_term_memory', content, details: { memorySave } }])
  expect(live.tools.call?.memorySave).toEqual(memorySave)
  expect(history.tools.call?.memorySave).toEqual(live.tools.call?.memorySave)
  expect(history.tools.call?.output).toEqual(live.tools.call?.output)
})

it('错误、部分输出和无效回执不能显示保存成功', () => {
  const failed = reduceRecord(emptyChat(), { type: 'tool_execution_end', toolCallId: 'call', isError: true, result: { content, details: { memorySave } } })
  expect(failed.tools.call?.memorySave).toBeUndefined()
  const partial = reduceRecord(emptyChat(), { type: 'tool_execution_update', toolCallId: 'call', partialResult: { details: { memorySave } } })
  expect(partial.tools.call?.memorySave).toBeUndefined()
  for (const value of [{ ...memorySave, revision: '2' }, { ...memorySave, outcome: 'ok' }, { ...memorySave, noteId: '' }]) {
    expect(restoreChat([{ role: 'toolResult', toolCallId: 'call', details: { memorySave: value } }]).tools.call?.memorySave).toBeUndefined()
  }
})

it('项目记忆更新提示仅来自结构有效的成功完成回执', () => {
  const projectMemoryUpdate = { outcome: 'saved', modId: 'p', revision: 3, changedFields: ['confirmedRequirements'] }
  const history = restoreChat([{ role: 'toolResult', toolCallId: 'call', content, details: { projectMemoryUpdate } }])
  const live = reduceRecord(emptyChat(), { type: 'tool_execution_end', toolCallId: 'call', result: { content, details: { projectMemoryUpdate } } })
  expect(history.tools.call?.projectMemoryUpdate).toEqual(projectMemoryUpdate)
  expect(live.tools.call?.projectMemoryUpdate).toEqual(projectMemoryUpdate)
  expect(restoreChat([{ role: 'toolResult', toolCallId: 'call', isError: true, details: { projectMemoryUpdate } }]).tools.call?.projectMemoryUpdate).toBeUndefined()
  expect(restoreChat([{ role: 'toolResult', toolCallId: 'call', details: { projectMemoryUpdate: { ...projectMemoryUpdate, changedFields: ['referenceSets'] } } }]).tools.call?.projectMemoryUpdate).toBeUndefined()
})

it('普通工具正文与命令回执仍可查看，助手声称已记住不生成回执', () => {
  const read = restoreChat([{ role: 'toolResult', toolCallId: 'read', content }])
  expect(read.tools.read?.output).toEqual(content)
  const details = { receiptId: 'receipt', status: 'failed' }
  const command = restoreChat([{ role: 'toolResult', toolCallId: 'command', content, details }])
  expect(command.tools.command?.output).toEqual({ content, details })
  expect(command.tools.command?.status).toBe('failed')
  expect(restoreChat([{ role: 'assistant', content: '已记住' }]).tools).toEqual({})
})
