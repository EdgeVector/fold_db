
import SelectField from '../../form/SelectField'
import { FORM_LABELS, MUTATION_TYPES } from '../../../constants/ui.js'
import { useAppSelector } from '../../../store/hooks'
import { selectApprovedSchemas } from '../../../store/schemaSlice'
import { buildSchemaOptions } from '../../../utils/schemaUtils'

function SchemaSelector({ selectedSchema, mutationType, onSchemaChange, onTypeChange }) {
  // Redux state
  const approvedSchemas = useAppSelector(selectApprovedSchemas)

  return (
    <div className="grid grid-cols-2 gap-4">
      <SelectField
        name="schema"
        label={FORM_LABELS.schema}
        value={selectedSchema}
        onChange={onSchemaChange}
        options={buildSchemaOptions(approvedSchemas)}
        placeholder="Select a schema..."
        emptyMessage="No approved schemas available for mutations"
        helpText={FORM_LABELS.schemaHelp}
      />

      <SelectField
        name="operationType"
        label={FORM_LABELS.operationType}
        value={mutationType}
        onChange={onTypeChange}
        options={MUTATION_TYPES}
        helpText={FORM_LABELS.operationHelp}
      />
    </div>
  )
}

export default SchemaSelector
