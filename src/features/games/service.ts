import type { Game } from '@/contracts/game'
import type { InspectionReport } from '@/contracts/inspection'
import { nativeCall } from '@/services/native'

export const gameService = {
  list(): Promise<Game[]> { return nativeCall('list_games') },
  cover(gameId: string): Promise<ArrayBuffer> { return nativeCall('read_game_cover', { gameId }) },
  icon(gameId: string): Promise<ArrayBuffer> { return nativeCall('read_game_icon', { gameId }) },
  inspections(modId: string): Promise<InspectionReport[]> { return nativeCall('list_inspections', { modId }) },
  pickDirectory(): Promise<string | null> { return nativeCall('pick_directory') },
  inspect(modId: string, kind: InspectionReport['kind'], directory: string): Promise<InspectionReport> {
    return nativeCall('inspect_supported_game', { modId, kind, directory })
  },
}
