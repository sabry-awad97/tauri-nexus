/**
 * Procedure and Router builders
 */

import type { ProcedureDef, ProcedureType, RouterDef } from './types';

export class ProcedureBuilder<TInput = void, TOutput = void> {
  private _command = '';
  private _type: ProcedureType = 'query';

  command(name: string): this {
    this._command = name;
    return this;
  }

  input<T>(): ProcedureBuilder<T, TOutput> {
    return this as unknown as ProcedureBuilder<T, TOutput>;
  }

  output<T>(): ProcedureBuilder<TInput, T> {
    return this as unknown as ProcedureBuilder<TInput, T>;
  }

  query(): ProcedureDef<TInput, TOutput> {
    return {
      _type: 'query',
      _input: undefined as TInput,
      _output: undefined as TOutput,
      _command: this._command,
    };
  }

  mutation(): ProcedureDef<TInput, TOutput> {
    return {
      _type: 'mutation',
      _input: undefined as TInput,
      _output: undefined as TOutput,
      _command: this._command,
    };
  }
}

export function procedure(): ProcedureBuilder<void, void> {
  return new ProcedureBuilder();
}

export function router<T extends RouterDef>(routes: T): T {
  return routes;
}
