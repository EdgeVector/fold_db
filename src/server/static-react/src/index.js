// Components
export { FoldDbProvider } from "./components/FoldDbProvider";
export { default as ResultsSection } from "./components/ResultsSection";
export { default as TabNavigation } from "./components/TabNavigation";
export { default as SettingsModal } from "./components/SettingsModal";
export { default as LogSidebar } from "./components/LogSidebar";
export { default as Header } from "./components/Header";
export { default as Footer } from "./components/Footer";
export { default as LoginPage } from "./components/LoginPage";
export { default as StructuredResults } from "./components/StructuredResults";
export { default as StatusSection } from "./components/StatusSection";

// Tab Components
export { default as SchemaTab } from "./components/tabs/SchemaTab";
export { default as QueryTab } from "./components/tabs/QueryTab";
export { default as LlmQueryTab } from "./components/tabs/LlmQueryTab";
export { default as MutationTab } from "./components/tabs/MutationTab";
export { default as IngestionTab } from "./components/tabs/IngestionTab";
export { default as FileUploadTab } from "./components/tabs/FileUploadTab";
export { default as NativeIndexTab } from "./components/tabs/NativeIndexTab";
export { default as KeyManagementTab } from "./components/tabs/KeyManagementTab";

// Schema Components
export { default as SchemaStatusBadge } from "./components/schema/SchemaStatusBadge";

// Form Components
export { default as FieldWrapper } from "./components/form/FieldWrapper";
export { default as RangeField } from "./components/form/RangeField";
export { default as SelectField } from "./components/form/SelectField";
export { default as TextField } from "./components/form/TextField";

// Query Components
export { default as QueryActions } from "./components/query/QueryActions";
export { default as QueryBuilder } from "./components/query/QueryBuilder";
export { default as QueryForm } from "./components/query/QueryForm";
export { default as QueryPreview } from "./components/query/QueryPreview";

// Mutation Components
export { default as MutationEditor } from "./components/tabs/mutation/MutationEditor";
export { default as ResultViewer } from "./components/tabs/mutation/ResultViewer";
export { default as SchemaSelector } from "./components/tabs/mutation/SchemaSelector";

// Hooks
export { useApprovedSchemas } from "./hooks/useApprovedSchemas";
export { useAppSelector, useAppDispatch } from "./store/hooks";

// Store (for Redux integration)
export { store } from "./store/store";
export { default as authReducer } from "./store/authSlice";
export { default as schemaReducer } from "./store/schemaSlice";
export { default as aiQueryReducer } from "./store/aiQuerySlice";

// Auth actions and thunks
export {
  initializeSystemKey,
  validatePrivateKey,
  refreshSystemKey,
  loginUser,
  clearAuthentication,
  setError,
  clearError,
  updateSystemKey,
  logoutUser,
  restoreSession,
} from "./store/authSlice";

// Constants
export { DEFAULT_TAB } from "./constants";

